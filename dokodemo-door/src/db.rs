use anyhow::{Context, Result};
use sqlx::{
    migrate::Migrator,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    PgPool, SqlitePool,
};
use std::str::FromStr;
use uuid::Uuid;

use crate::error::ApiError;
use crate::helpers::serialize_tags;
use crate::models::{Entry, EntryRow, NewEntry};

pub static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("migrations/postgres");
pub static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("migrations/sqlite");

#[derive(Clone)]
pub enum Database {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

pub async fn connect_database(database_url: &str) -> Result<Database> {
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

pub async fn run_migrations(db: &Database) -> Result<()> {
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

pub async fn insert_entry(
    db: &Database,
    data: NewEntry,
    tags: &[String],
) -> Result<Entry, ApiError> {
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

pub async fn fetch_entry(db: &Database, id: &str) -> Result<Entry, ApiError> {
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

pub async fn sync_entry_tags(
    db: &Database,
    entry_id: &str,
    tags: &[String],
) -> Result<(), ApiError> {
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
