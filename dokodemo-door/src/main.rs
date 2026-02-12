use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, net::SocketAddr};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

const ALLOWED_KINDS: &[&str] = &[
    "book",
    "manga",
    "article",
    "animation",
    "movie",
    "series",
    "note",
    "link",
];

const ALLOWED_STATUSES: &[&str] = &["planned", "in_progress", "completed", "dropped"];

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    telegram_webhook_secret: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct Entry {
    id: Uuid,
    title: String,
    kind: String,
    status: String,
    notes: String,
    url: Option<String>,
    source: String,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateEntryRequest {
    title: String,
    kind: String,
    status: Option<String>,
    notes: Option<String>,
    url: Option<String>,
    source: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UpdateEntryRequest {
    title: Option<String>,
    kind: Option<String>,
    status: Option<String>,
    notes: Option<String>,
    url: Option<String>,
    source: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ListEntriesQuery {
    kind: Option<String>,
    status: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct QuickCaptureRequest {
    text: String,
    title: Option<String>,
    kind: Option<String>,
    status: Option<String>,
    source: Option<String>,
    tags: Option<Vec<String>>,
    url: Option<String>,
}

#[derive(Debug)]
struct NewEntry {
    title: String,
    kind: String,
    status: String,
    notes: String,
    url: Option<String>,
    source: String,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    message: Option<TelegramMessage>,
    edited_message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    text: Option<String>,
    caption: Option<String>,
    chat: TelegramChat,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct AcceptedResponse {
    status: &'static str,
    entry_id: Uuid,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::NotFound => (StatusCode::NOT_FOUND, "resource not found".to_string()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            Self::Database(err) => {
                tracing::error!(error = ?err, "database failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dokodemo_door=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let telegram_webhook_secret = env::var("TELEGRAM_WEBHOOK_SECRET").ok();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    ensure_schema(&pool).await?;

    let state = AppState {
        pool,
        telegram_webhook_secret,
    };

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api/v1", api_routes())
        .layer(
            ServiceBuilder::new()
                .layer(CompressionLayer::new())
                .layer(TraceLayer::new_for_http()),
        )
        .with_state(state);

    let addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("invalid BIND_ADDR: {bind_addr}"))?;

    tracing::info!(%addr, "dokodemo-door listening");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("server failed")?;

    Ok(())
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/entries", post(create_entry).get(list_entries))
        .route(
            "/entries/:id",
            get(get_entry).patch(update_entry).delete(delete_entry),
        )
        .route("/quick-capture", post(quick_capture))
        .route("/integrations/telegram/update", post(telegram_capture))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(include_str!("../sql/001_init.sql"))
        .execute(pool)
        .await
        .context("failed to apply bootstrap schema")?;
    Ok(())
}

async fn create_entry(
    State(state): State<AppState>,
    Json(req): Json<CreateEntryRequest>,
) -> Result<(StatusCode, Json<Entry>), ApiError> {
    validate_kind(&req.kind)?;

    let status = req.status.unwrap_or_else(|| "planned".to_string());
    validate_status(&status)?;

    let entry = insert_entry(
        &state.pool,
        NewEntry {
            title: req.title,
            kind: req.kind,
            status,
            notes: req.notes.unwrap_or_default(),
            url: req.url,
            source: req.source.unwrap_or_else(|| "manual".to_string()),
            tags: req.tags.unwrap_or_default(),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(entry)))
}

async fn list_entries(
    State(state): State<AppState>,
    Query(query): Query<ListEntriesQuery>,
) -> Result<Json<Vec<Entry>>, ApiError> {
    if let Some(kind) = &query.kind {
        validate_kind(kind)?;
    }
    if let Some(status) = &query.status {
        validate_status(status)?;
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let entries = sqlx::query_as::<_, Entry>(
        r#"
        SELECT id, title, kind, status, notes, url, source, tags, created_at, updated_at
        FROM entries
        WHERE ($1::text IS NULL OR kind = $1)
          AND ($2::text IS NULL OR status = $2)
          AND (
            $3::text IS NULL
            OR title ILIKE '%' || $3 || '%'
            OR notes ILIKE '%' || $3 || '%'
          )
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(query.kind)
    .bind(query.status)
    .bind(query.search)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(entries))
}

async fn get_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Entry>, ApiError> {
    let entry = sqlx::query_as::<_, Entry>(
        r#"
        SELECT id, title, kind, status, notes, url, source, tags, created_at, updated_at
        FROM entries
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(entry))
}

async fn update_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEntryRequest>,
) -> Result<Json<Entry>, ApiError> {
    if let Some(kind) = &req.kind {
        validate_kind(kind)?;
    }
    if let Some(status) = &req.status {
        validate_status(status)?;
    }

    let updated = sqlx::query_as::<_, Entry>(
        r#"
        UPDATE entries
        SET
          title = COALESCE($2, title),
          kind = COALESCE($3, kind),
          status = COALESCE($4, status),
          notes = COALESCE($5, notes),
          url = COALESCE($6, url),
          source = COALESCE($7, source),
          tags = COALESCE($8, tags),
          updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, kind, status, notes, url, source, tags, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(req.title)
    .bind(req.kind)
    .bind(req.status)
    .bind(req.notes)
    .bind(req.url)
    .bind(req.source)
    .bind(req.tags)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(updated))
}

async fn delete_entry(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM entries WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn quick_capture(
    State(state): State<AppState>,
    Json(req): Json<QuickCaptureRequest>,
) -> Result<(StatusCode, Json<Entry>), ApiError> {
    let kind = req.kind.unwrap_or_else(|| "note".to_string());
    validate_kind(&kind)?;

    let status = req.status.unwrap_or_else(|| "planned".to_string());
    validate_status(&status)?;

    let title = req.title.unwrap_or_else(|| summarize_title(&req.text));
    let url = req.url.or_else(|| extract_url_from_text(&req.text));

    let entry = insert_entry(
        &state.pool,
        NewEntry {
            title,
            kind,
            status,
            notes: req.text,
            url,
            source: req.source.unwrap_or_else(|| "quick-capture".to_string()),
            tags: req.tags.unwrap_or_default(),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(entry)))
}

async fn telegram_capture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<TelegramUpdate>,
) -> Result<(StatusCode, Json<AcceptedResponse>), ApiError> {
    if let Some(expected) = &state.telegram_webhook_secret {
        let header = headers
            .get("x-telegram-bot-api-secret-token")
            .and_then(|value| value.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        if header != expected {
            return Err(ApiError::Unauthorized);
        }
    }

    let message = update.message.or(update.edited_message).ok_or_else(|| {
        ApiError::BadRequest("telegram update does not contain a message payload".to_string())
    })?;

    let text = message.text.or(message.caption).ok_or_else(|| {
        ApiError::BadRequest("telegram message does not contain text".to_string())
    })?;

    let title = summarize_title(&text);
    let url = extract_url_from_text(&text);
    let source = format!("telegram:{}", message.chat.id);

    let entry = insert_entry(
        &state.pool,
        NewEntry {
            title,
            kind: "note".to_string(),
            status: "planned".to_string(),
            notes: text,
            url,
            source,
            tags: Vec::new(),
        },
    )
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(AcceptedResponse {
            status: "accepted",
            entry_id: entry.id,
        }),
    ))
}

async fn insert_entry(pool: &PgPool, data: NewEntry) -> Result<Entry, ApiError> {
    let entry = sqlx::query_as::<_, Entry>(
        r#"
        INSERT INTO entries (id, title, kind, status, notes, url, source, tags)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, title, kind, status, notes, url, source, tags, created_at, updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(data.title)
    .bind(data.kind)
    .bind(data.status)
    .bind(data.notes)
    .bind(data.url)
    .bind(data.source)
    .bind(data.tags)
    .fetch_one(pool)
    .await?;

    Ok(entry)
}

fn summarize_title(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or_default().trim();
    let mut title = first_line.chars().take(80).collect::<String>();
    if title.is_empty() {
        title = "quick note".to_string();
    }
    title
}

fn extract_url_from_text(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.starts_with("http://") || part.starts_with("https://"))
        .map(|value| {
            value
                .trim_end_matches(&[')', ']', '}', ',', '.', ';'][..])
                .to_string()
        })
}

fn validate_kind(kind: &str) -> Result<(), ApiError> {
    if ALLOWED_KINDS.contains(&kind) {
        return Ok(());
    }

    Err(ApiError::BadRequest(format!(
        "invalid kind `{kind}`, allowed: {}",
        ALLOWED_KINDS.join(", ")
    )))
}

fn validate_status(status: &str) -> Result<(), ApiError> {
    if ALLOWED_STATUSES.contains(&status) {
        return Ok(());
    }

    Err(ApiError::BadRequest(format!(
        "invalid status `{status}`, allowed: {}",
        ALLOWED_STATUSES.join(", ")
    )))
}
