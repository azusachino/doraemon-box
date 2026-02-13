use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::db::Database;
use crate::error::ApiError;
use crate::helpers::map_unique_error;
use crate::models::{Category, CategoryRow, CreateCategoryRequest, UpdateCategoryRequest};
use crate::AppState;

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<Category>>, ApiError> {
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

pub async fn create_category(
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

pub async fn update_category(
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

pub async fn delete_category(
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
