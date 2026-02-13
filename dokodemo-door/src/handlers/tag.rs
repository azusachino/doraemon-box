use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::db::Database;
use crate::error::ApiError;
use crate::helpers::map_unique_error;
use crate::models::{CreateTagRequest, Tag, TagRow};
use crate::AppState;

pub async fn list_tags(State(state): State<AppState>) -> Result<Json<Vec<Tag>>, ApiError> {
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

pub async fn create_tag(
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

pub async fn delete_tag(
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
