use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use uuid::Uuid;

use crate::db::{fetch_entry, insert_entry, sync_entry_tags, Database};
use crate::error::ApiError;
use crate::helpers::{
    extract_url_from_text, serialize_tags, summarize_title, validate_kind, validate_status,
};
use crate::models::{
    AcceptedResponse, CreateEntryRequest, Entry, EntryRow, ListEntriesQuery, NewEntry,
    QuickCaptureRequest, TelegramUpdate, UpdateEntryRequest,
};
use crate::AppState;

pub async fn create_entry(
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

pub async fn list_entries(
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

pub async fn get_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Entry>, ApiError> {
    Ok(Json(fetch_entry(&state.db, &id).await?))
}

pub async fn update_entry(
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

pub async fn delete_entry(
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

pub async fn quick_capture(
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

pub async fn telegram_capture(
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
