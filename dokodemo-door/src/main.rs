use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    migrate::Migrator,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    FromRow, PgPool, SqlitePool,
};
use std::{env, net::SocketAddr, str::FromStr};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

const ALLOWED_STATUSES: &[&str] = &["planned", "in_progress", "completed", "dropped"];
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("migrations/postgres");
static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("migrations/sqlite");

#[derive(Clone)]
struct AppState {
    db: Database,
    telegram_webhook_secret: Option<String>,
    api_key: Option<String>,
}

#[derive(Clone)]
enum Database {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

// -- Entry types --

#[derive(Debug, Serialize)]
struct Entry {
    id: String,
    title: String,
    kind: String,
    status: String,
    notes: String,
    url: Option<String>,
    source: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct EntryRow {
    id: String,
    title: String,
    kind: String,
    status: String,
    notes: String,
    url: Option<String>,
    source: String,
    tags_json: String,
    created_at: String,
    updated_at: String,
}

impl EntryRow {
    fn into_entry(self) -> Result<Entry, ApiError> {
        let tags = serde_json::from_str::<Vec<String>>(&self.tags_json).map_err(|err| {
            ApiError::Internal(format!("invalid tags payload in database: {err}"))
        })?;

        Ok(Entry {
            id: self.id,
            title: self.title,
            kind: self.kind,
            status: self.status,
            notes: self.notes,
            url: self.url,
            source: self.source,
            tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// -- Category types --

#[derive(Debug, Serialize)]
struct Category {
    id: String,
    name: String,
    description: String,
    created_at: String,
}

#[derive(Debug, FromRow)]
struct CategoryRow {
    id: String,
    name: String,
    description: String,
    created_at: String,
}

impl CategoryRow {
    fn into_category(self) -> Category {
        Category {
            id: self.id,
            name: self.name,
            description: self.description,
            created_at: self.created_at,
        }
    }
}

// -- Tag types --

#[derive(Debug, Serialize)]
struct Tag {
    id: String,
    name: String,
    created_at: String,
}

#[derive(Debug, FromRow)]
struct TagRow {
    id: String,
    name: String,
    created_at: String,
}

impl TagRow {
    fn into_tag(self) -> Tag {
        Tag {
            id: self.id,
            name: self.name,
            created_at: self.created_at,
        }
    }
}

// -- Request types --

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
    tag: Option<String>,
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
    id: String,
    title: String,
    kind: String,
    status: String,
    notes: String,
    url: Option<String>,
    source: String,
    tags_json: String,
}

#[derive(Debug, Deserialize)]
struct CreateCategoryRequest {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateCategoryRequest {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTagRequest {
    name: String,
}

// -- Telegram types --

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

// -- Response types --

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
}

#[derive(Debug, Serialize)]
struct AcceptedResponse {
    status: &'static str,
    entry_id: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// -- Error --

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
    #[error("internal error: {0}")]
    Internal(String),
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
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "internal failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

// -- Main --

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dokodemo_door=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./data/doraemon-box.db?mode=rwc".to_string());
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let telegram_webhook_secret = env::var("TELEGRAM_WEBHOOK_SECRET").ok();
    let api_key = env::var("APP_API_KEY").ok();
    let migrate_only = env::args().any(|arg| arg == "--migrate-only");

    let db = connect_database(&database_url).await?;
    run_migrations(&db).await?;

    if migrate_only {
        tracing::info!("migrations completed");
        return Ok(());
    }

    let state = AppState {
        db,
        telegram_webhook_secret,
        api_key,
    };

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api/v1", api_routes(state.clone()))
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

fn api_routes(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/entries", post(create_entry).get(list_entries))
        .route(
            "/entries/:id",
            get(get_entry).patch(update_entry).delete(delete_entry),
        )
        .route("/quick-capture", post(quick_capture))
        .route("/categories", post(create_category).get(list_categories))
        .route(
            "/categories/:id",
            patch(update_category).delete(delete_category),
        )
        .route("/tags", post(create_tag).get(list_tags))
        .route("/tags/:id", delete(delete_tag))
        .layer(from_fn_with_state(state, api_key_auth));

    Router::new()
        .merge(protected_routes)
        .route("/integrations/telegram/update", post(telegram_capture))
}

// -- Database setup --

async fn connect_database(database_url: &str) -> Result<Database> {
    if database_url.starts_with("sqlite:") {
        let options = SqliteConnectOptions::from_str(database_url)
            .with_context(|| format!("invalid sqlite DATABASE_URL: {database_url}"))?;

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("failed to connect to sqlite")?;

        return Ok(Database::Sqlite(pool));
    }

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    Ok(Database::Postgres(pool))
}

async fn run_migrations(db: &Database) -> Result<()> {
    match db {
        Database::Postgres(pool) => {
            POSTGRES_MIGRATOR
                .run(pool)
                .await
                .context("failed to run postgres migrations")?;
        }
        Database::Sqlite(pool) => {
            SQLITE_MIGRATOR
                .run(pool)
                .await
                .context("failed to run sqlite migrations")?;
        }
    }

    Ok(())
}

// -- Auth middleware --

async fn api_key_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next<axum::body::Body>,
) -> Result<Response, ApiError> {
    let Some(expected_api_key) = state.api_key.as_deref() else {
        return Ok(next.run(request).await);
    };

    let bearer_key = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let header_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let provided = bearer_key.or(header_key);

    match provided {
        Some(value) if value == expected_api_key => Ok(next.run(request).await),
        _ => Err(ApiError::Unauthorized),
    }
}

// -- Health --

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let database = match state.db {
        Database::Postgres(_) => "postgres",
        Database::Sqlite(_) => "sqlite",
    };

    Json(HealthResponse {
        status: "ok",
        database,
    })
}

// -- Entry handlers --

async fn create_entry(
    State(state): State<AppState>,
    Json(req): Json<CreateEntryRequest>,
) -> Result<(StatusCode, Json<Entry>), ApiError> {
    validate_kind(&state.db, &req.kind).await?;

    let status = req.status.unwrap_or_else(|| "planned".to_string());
    validate_status(&status)?;

    let tags = req.tags.unwrap_or_default();
    let entry = insert_entry(
        &state.db,
        NewEntry {
            id: Uuid::new_v4().to_string(),
            title: req.title,
            kind: req.kind,
            status,
            notes: req.notes.unwrap_or_default(),
            url: req.url,
            source: req.source.unwrap_or_else(|| "manual".to_string()),
            tags_json: serialize_tags(tags.clone())?,
        },
        &tags,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(entry)))
}

async fn list_entries(
    State(state): State<AppState>,
    Query(query): Query<ListEntriesQuery>,
) -> Result<Json<Vec<Entry>>, ApiError> {
    if let Some(kind) = &query.kind {
        validate_kind(&state.db, kind).await?;
    }
    if let Some(status) = &query.status {
        validate_status(status)?;
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let rows = match &state.db {
        Database::Postgres(pool) => {
            sqlx::query_as::<_, EntryRow>(
                r#"
                SELECT
                  e.id,
                  e.title,
                  e.kind,
                  e.status,
                  e.notes,
                  e.url,
                  e.source,
                  COALESCE(
                    (SELECT json_agg(sub.name)::text
                     FROM (SELECT t.name
                           FROM entry_tags et
                           JOIN tags t ON t.id = et.tag_id
                           WHERE et.entry_id = e.id
                           ORDER BY t.name) sub),
                    '[]'
                  ) AS tags_json,
                  e.created_at::text AS created_at,
                  e.updated_at::text AS updated_at
                FROM entries e
                WHERE ($1::text IS NULL OR e.kind = $1)
                  AND ($2::text IS NULL OR e.status = $2)
                  AND (
                    $3::text IS NULL
                    OR LOWER(e.title) LIKE '%' || LOWER($3) || '%'
                    OR LOWER(e.notes) LIKE '%' || LOWER($3) || '%'
                  )
                  AND (
                    $4::text IS NULL
                    OR EXISTS (
                      SELECT 1 FROM entry_tags et2
                      JOIN tags t2 ON t2.id = et2.tag_id
                      WHERE et2.entry_id = e.id AND t2.name = $4
                    )
                  )
                ORDER BY e.created_at DESC
                LIMIT $5 OFFSET $6
                "#,
            )
            .bind(&query.kind)
            .bind(&query.status)
            .bind(&query.search)
            .bind(&query.tag)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_as::<_, EntryRow>(
                r#"
                SELECT
                  e.id,
                  e.title,
                  e.kind,
                  e.status,
                  e.notes,
                  e.url,
                  e.source,
                  COALESCE(
                    (SELECT json_group_array(sub.name)
                     FROM (SELECT t.name
                           FROM entry_tags et
                           JOIN tags t ON t.id = et.tag_id
                           WHERE et.entry_id = e.id
                           ORDER BY t.name) sub),
                    '[]'
                  ) AS tags_json,
                  e.created_at,
                  e.updated_at
                FROM entries e
                WHERE (?1 IS NULL OR e.kind = ?1)
                  AND (?2 IS NULL OR e.status = ?2)
                  AND (
                    ?3 IS NULL
                    OR LOWER(e.title) LIKE '%' || LOWER(?3) || '%'
                    OR LOWER(e.notes) LIKE '%' || LOWER(?3) || '%'
                  )
                  AND (
                    ?4 IS NULL
                    OR EXISTS (
                      SELECT 1 FROM entry_tags et2
                      JOIN tags t2 ON t2.id = et2.tag_id
                      WHERE et2.entry_id = e.id AND t2.name = ?4
                    )
                  )
                ORDER BY e.created_at DESC
                LIMIT ?5 OFFSET ?6
                "#,
            )
            .bind(&query.kind)
            .bind(&query.status)
            .bind(&query.search)
            .bind(&query.tag)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };

    let entries = rows
        .into_iter()
        .map(EntryRow::into_entry)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(entries))
}

async fn get_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Entry>, ApiError> {
    Ok(Json(fetch_entry(&state.db, &id).await?))
}

async fn update_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateEntryRequest>,
) -> Result<Json<Entry>, ApiError> {
    if let Some(kind) = &req.kind {
        validate_kind(&state.db, kind).await?;
    }
    if let Some(status) = &req.status {
        validate_status(status)?;
    }

    let tags_json = match &req.tags {
        Some(tags) => Some(serialize_tags(tags.clone())?),
        None => None,
    };

    let rows_affected = match &state.db {
        Database::Postgres(pool) => sqlx::query(
            r#"
                UPDATE entries
                SET
                  title = COALESCE($2, title),
                  kind = COALESCE($3, kind),
                  status = COALESCE($4, status),
                  notes = COALESCE($5, notes),
                  url = COALESCE($6, url),
                  source = COALESCE($7, source),
                  tags_json = COALESCE($8, tags_json),
                  updated_at = NOW()
                WHERE id = $1
                "#,
        )
        .bind(&id)
        .bind(&req.title)
        .bind(&req.kind)
        .bind(&req.status)
        .bind(&req.notes)
        .bind(&req.url)
        .bind(&req.source)
        .bind(&tags_json)
        .execute(pool)
        .await?
        .rows_affected(),
        Database::Sqlite(pool) => sqlx::query(
            r#"
                UPDATE entries
                SET
                  title = COALESCE(?2, title),
                  kind = COALESCE(?3, kind),
                  status = COALESCE(?4, status),
                  notes = COALESCE(?5, notes),
                  url = COALESCE(?6, url),
                  source = COALESCE(?7, source),
                  tags_json = COALESCE(?8, tags_json),
                  updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                WHERE id = ?1
                "#,
        )
        .bind(&id)
        .bind(&req.title)
        .bind(&req.kind)
        .bind(&req.status)
        .bind(&req.notes)
        .bind(&req.url)
        .bind(&req.source)
        .bind(&tags_json)
        .execute(pool)
        .await?
        .rows_affected(),
    };

    if rows_affected == 0 {
        return Err(ApiError::NotFound);
    }

    if let Some(tags) = &req.tags {
        sync_entry_tags(&state.db, &id, tags).await?;
    }

    Ok(Json(fetch_entry(&state.db, &id).await?))
}

async fn delete_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let rows_affected = match &state.db {
        Database::Postgres(pool) => sqlx::query("DELETE FROM entries WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
        Database::Sqlite(pool) => sqlx::query("DELETE FROM entries WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected(),
    };

    if rows_affected == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn quick_capture(
    State(state): State<AppState>,
    Json(req): Json<QuickCaptureRequest>,
) -> Result<(StatusCode, Json<Entry>), ApiError> {
    let kind = req.kind.unwrap_or_else(|| "note".to_string());
    validate_kind(&state.db, &kind).await?;

    let status = req.status.unwrap_or_else(|| "planned".to_string());
    validate_status(&status)?;

    let title = req.title.unwrap_or_else(|| summarize_title(&req.text));
    let url = req.url.or_else(|| extract_url_from_text(&req.text));
    let tags = req.tags.unwrap_or_default();

    let entry = insert_entry(
        &state.db,
        NewEntry {
            id: Uuid::new_v4().to_string(),
            title,
            kind,
            status,
            notes: req.text,
            url,
            source: req.source.unwrap_or_else(|| "quick-capture".to_string()),
            tags_json: serialize_tags(tags.clone())?,
        },
        &tags,
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
        &state.db,
        NewEntry {
            id: Uuid::new_v4().to_string(),
            title,
            kind: "note".to_string(),
            status: "planned".to_string(),
            notes: text,
            url,
            source,
            tags_json: serialize_tags(Vec::new())?,
        },
        &[],
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

// -- Category handlers --

async fn list_categories(State(state): State<AppState>) -> Result<Json<Vec<Category>>, ApiError> {
    let rows = match &state.db {
        Database::Postgres(pool) => {
            sqlx::query_as::<_, CategoryRow>(
                "SELECT id, name, description, \
                 created_at::text AS created_at \
                 FROM categories ORDER BY name",
            )
            .fetch_all(pool)
            .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_as::<_, CategoryRow>(
                "SELECT id, name, description, created_at \
                 FROM categories ORDER BY name",
            )
            .fetch_all(pool)
            .await?
        }
    };

    Ok(Json(
        rows.into_iter().map(CategoryRow::into_category).collect(),
    ))
}

async fn create_category(
    State(state): State<AppState>,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<Category>), ApiError> {
    let name = req.name.trim().to_lowercase();
    if name.is_empty() {
        return Err(ApiError::BadRequest(
            "category name cannot be empty".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let description = req.description.unwrap_or_default();

    let row = match &state.db {
        Database::Postgres(pool) => sqlx::query_as::<_, CategoryRow>(
            "INSERT INTO categories (id, name, description) \
             VALUES ($1, $2, $3) \
             RETURNING id, name, description, \
             created_at::text AS created_at",
        )
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .fetch_one(pool)
        .await
        .map_err(|e| map_unique_error(e, &name, "category"))?,
        Database::Sqlite(pool) => sqlx::query_as::<_, CategoryRow>(
            "INSERT INTO categories (id, name, description) \
             VALUES (?1, ?2, ?3) \
             RETURNING id, name, description, created_at",
        )
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .fetch_one(pool)
        .await
        .map_err(|e| map_unique_error(e, &name, "category"))?,
    };

    Ok((StatusCode::CREATED, Json(row.into_category())))
}

async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<Json<Category>, ApiError> {
    let name = req
        .name
        .map(|n| {
            let trimmed = n.trim().to_lowercase();
            if trimmed.is_empty() {
                Err(ApiError::BadRequest(
                    "category name cannot be empty".to_string(),
                ))
            } else {
                Ok(trimmed)
            }
        })
        .transpose()?;

    let row = match &state.db {
        Database::Postgres(pool) => {
            sqlx::query_as::<_, CategoryRow>(
                "UPDATE categories \
             SET name = COALESCE($2, name), \
                 description = COALESCE($3, description) \
             WHERE id = $1 \
             RETURNING id, name, description, \
             created_at::text AS created_at",
            )
            .bind(&id)
            .bind(&name)
            .bind(&req.description)
            .fetch_optional(pool)
            .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_as::<_, CategoryRow>(
                "UPDATE categories \
             SET name = COALESCE(?2, name), \
                 description = COALESCE(?3, description) \
             WHERE id = ?1 \
             RETURNING id, name, description, created_at",
            )
            .bind(&id)
            .bind(&name)
            .bind(&req.description)
            .fetch_optional(pool)
            .await?
        }
    }
    .ok_or(ApiError::NotFound)?;

    Ok(Json(row.into_category()))
}

async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let in_use = match &state.db {
        Database::Postgres(pool) => {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(\
               SELECT 1 FROM entries e \
               JOIN categories c ON c.name = e.kind \
               WHERE c.id = $1\
             )",
            )
            .bind(&id)
            .fetch_one(pool)
            .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(\
               SELECT 1 FROM entries e \
               JOIN categories c ON c.name = e.kind \
               WHERE c.id = ?1\
             )",
            )
            .bind(&id)
            .fetch_one(pool)
            .await?
        }
    };

    if in_use {
        return Err(ApiError::BadRequest(
            "cannot delete category that is in use by entries".to_string(),
        ));
    }

    let rows_affected = match &state.db {
        Database::Postgres(pool) => sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(&id)
            .execute(pool)
            .await?
            .rows_affected(),
        Database::Sqlite(pool) => sqlx::query("DELETE FROM categories WHERE id = ?1")
            .bind(&id)
            .execute(pool)
            .await?
            .rows_affected(),
    };

    if rows_affected == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

// -- Tag handlers --

async fn list_tags(State(state): State<AppState>) -> Result<Json<Vec<Tag>>, ApiError> {
    let rows = match &state.db {
        Database::Postgres(pool) => {
            sqlx::query_as::<_, TagRow>(
                "SELECT id, name, created_at::text AS created_at \
                 FROM tags ORDER BY name",
            )
            .fetch_all(pool)
            .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_as::<_, TagRow>(
                "SELECT id, name, created_at \
                 FROM tags ORDER BY name",
            )
            .fetch_all(pool)
            .await?
        }
    };

    Ok(Json(rows.into_iter().map(TagRow::into_tag).collect()))
}

async fn create_tag(
    State(state): State<AppState>,
    Json(req): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<Tag>), ApiError> {
    let name = req.name.trim().to_lowercase();
    if name.is_empty() {
        return Err(ApiError::BadRequest("tag name cannot be empty".to_string()));
    }

    let id = Uuid::new_v4().to_string();

    let row = match &state.db {
        Database::Postgres(pool) => sqlx::query_as::<_, TagRow>(
            "INSERT INTO tags (id, name) VALUES ($1, $2) \
             RETURNING id, name, created_at::text AS created_at",
        )
        .bind(&id)
        .bind(&name)
        .fetch_one(pool)
        .await
        .map_err(|e| map_unique_error(e, &name, "tag"))?,
        Database::Sqlite(pool) => sqlx::query_as::<_, TagRow>(
            "INSERT INTO tags (id, name) VALUES (?1, ?2) \
             RETURNING id, name, created_at",
        )
        .bind(&id)
        .bind(&name)
        .fetch_one(pool)
        .await
        .map_err(|e| map_unique_error(e, &name, "tag"))?,
    };

    Ok((StatusCode::CREATED, Json(row.into_tag())))
}

async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let rows_affected = match &state.db {
        Database::Postgres(pool) => sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(&id)
            .execute(pool)
            .await?
            .rows_affected(),
        Database::Sqlite(pool) => sqlx::query("DELETE FROM tags WHERE id = ?1")
            .bind(&id)
            .execute(pool)
            .await?
            .rows_affected(),
    };

    if rows_affected == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

// -- Helpers --

async fn insert_entry(db: &Database, data: NewEntry, tags: &[String]) -> Result<Entry, ApiError> {
    match db {
        Database::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO entries \
                 (id, title, kind, status, notes, url, source, tags_json) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&data.id)
            .bind(&data.title)
            .bind(&data.kind)
            .bind(&data.status)
            .bind(&data.notes)
            .bind(&data.url)
            .bind(&data.source)
            .bind(&data.tags_json)
            .execute(pool)
            .await?;
        }
        Database::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO entries \
                 (id, title, kind, status, notes, url, source, tags_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .bind(&data.id)
            .bind(&data.title)
            .bind(&data.kind)
            .bind(&data.status)
            .bind(&data.notes)
            .bind(&data.url)
            .bind(&data.source)
            .bind(&data.tags_json)
            .execute(pool)
            .await?;
        }
    }

    if !tags.is_empty() {
        sync_entry_tags(db, &data.id, tags).await?;
    }

    fetch_entry(db, &data.id).await
}

async fn fetch_entry(db: &Database, id: &str) -> Result<Entry, ApiError> {
    let row = match db {
        Database::Postgres(pool) => {
            sqlx::query_as::<_, EntryRow>(
                r#"
                SELECT
                  e.id,
                  e.title,
                  e.kind,
                  e.status,
                  e.notes,
                  e.url,
                  e.source,
                  COALESCE(
                    (SELECT json_agg(sub.name)::text
                     FROM (SELECT t.name
                           FROM entry_tags et
                           JOIN tags t ON t.id = et.tag_id
                           WHERE et.entry_id = e.id
                           ORDER BY t.name) sub),
                    '[]'
                  ) AS tags_json,
                  e.created_at::text AS created_at,
                  e.updated_at::text AS updated_at
                FROM entries e
                WHERE e.id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_as::<_, EntryRow>(
                r#"
                SELECT
                  e.id,
                  e.title,
                  e.kind,
                  e.status,
                  e.notes,
                  e.url,
                  e.source,
                  COALESCE(
                    (SELECT json_group_array(sub.name)
                     FROM (SELECT t.name
                           FROM entry_tags et
                           JOIN tags t ON t.id = et.tag_id
                           WHERE et.entry_id = e.id
                           ORDER BY t.name) sub),
                    '[]'
                  ) AS tags_json,
                  e.created_at,
                  e.updated_at
                FROM entries e
                WHERE e.id = ?1
                "#,
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
        }
    }
    .ok_or(ApiError::NotFound)?;

    row.into_entry()
}

async fn sync_entry_tags(db: &Database, entry_id: &str, tags: &[String]) -> Result<(), ApiError> {
    let lowered: Vec<String> = tags.iter().map(|t| t.trim().to_lowercase()).collect();

    match db {
        Database::Postgres(pool) => {
            for tag_name in &lowered {
                sqlx::query(
                    "INSERT INTO tags (id, name) VALUES ($1, $2) \
                     ON CONFLICT (name) DO NOTHING",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(tag_name)
                .execute(pool)
                .await?;
            }

            sqlx::query("DELETE FROM entry_tags WHERE entry_id = $1")
                .bind(entry_id)
                .execute(pool)
                .await?;

            for tag_name in &lowered {
                sqlx::query(
                    "INSERT INTO entry_tags (entry_id, tag_id) \
                     SELECT $1, id FROM tags WHERE name = $2",
                )
                .bind(entry_id)
                .bind(tag_name)
                .execute(pool)
                .await?;
            }

            let tags_json = serialize_tags(lowered.clone())?;
            sqlx::query("UPDATE entries SET tags_json = $1 WHERE id = $2")
                .bind(&tags_json)
                .bind(entry_id)
                .execute(pool)
                .await?;
        }
        Database::Sqlite(pool) => {
            for tag_name in &lowered {
                sqlx::query(
                    "INSERT OR IGNORE INTO tags (id, name) \
                     VALUES (?1, ?2)",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(tag_name)
                .execute(pool)
                .await?;
            }

            sqlx::query("DELETE FROM entry_tags WHERE entry_id = ?1")
                .bind(entry_id)
                .execute(pool)
                .await?;

            for tag_name in &lowered {
                sqlx::query(
                    "INSERT INTO entry_tags (entry_id, tag_id) \
                     SELECT ?1, id FROM tags WHERE name = ?2",
                )
                .bind(entry_id)
                .bind(tag_name)
                .execute(pool)
                .await?;
            }

            let tags_json = serialize_tags(lowered.clone())?;
            sqlx::query("UPDATE entries SET tags_json = ?1 WHERE id = ?2")
                .bind(&tags_json)
                .bind(entry_id)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

fn serialize_tags(tags: Vec<String>) -> Result<String, ApiError> {
    serde_json::to_string(&tags)
        .map_err(|err| ApiError::Internal(format!("failed to serialize tags: {err}")))
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

async fn validate_kind(db: &Database, kind: &str) -> Result<(), ApiError> {
    let exists = match db {
        Database::Postgres(pool) => {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM categories WHERE name = $1)")
                .bind(kind)
                .fetch_one(pool)
                .await?
        }
        Database::Sqlite(pool) => {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM categories WHERE name = ?1)")
                .bind(kind)
                .fetch_one(pool)
                .await?
        }
    };

    if !exists {
        return Err(ApiError::BadRequest(format!(
            "invalid kind `{kind}`, not found in categories"
        )));
    }

    Ok(())
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

fn map_unique_error(e: sqlx::Error, name: &str, entity: &str) -> ApiError {
    if let sqlx::Error::Database(ref db_err) = e {
        let msg = db_err.message().to_lowercase();
        if msg.contains("unique") || msg.contains("duplicate") {
            return ApiError::BadRequest(format!("{entity} `{name}` already exists"));
        }
    }
    ApiError::Database(e)
}

#[cfg(test)]
mod tests {
    use super::{extract_url_from_text, summarize_title};

    #[test]
    fn summarize_title_uses_first_line() {
        let input = "Read Pluto vol.1\nGreat pacing";
        assert_eq!(summarize_title(input), "Read Pluto vol.1");
    }

    #[test]
    fn extract_url_handles_trailing_punctuation() {
        let input = "save this https://example.com/path?x=1, thanks";
        assert_eq!(
            extract_url_from_text(input),
            Some("https://example.com/path?x=1".to_string())
        );
    }
}
