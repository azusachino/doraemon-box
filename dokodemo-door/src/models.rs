use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::error::ApiError;

// -- Entry types --

#[derive(Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub notes: String,
    pub url: Option<String>,
    pub source: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, FromRow)]
pub struct EntryRow {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub notes: String,
    pub url: Option<String>,
    pub source: String,
    pub tags_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl EntryRow {
    pub fn into_entry(self) -> Result<Entry, ApiError> {
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
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub struct CategoryRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

impl CategoryRow {
    pub fn into_category(self) -> Category {
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
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub struct TagRow {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

impl TagRow {
    pub fn into_tag(self) -> Tag {
        Tag {
            id: self.id,
            name: self.name,
            created_at: self.created_at,
        }
    }
}

// -- Request types --

#[derive(Debug, Deserialize)]
pub struct CreateEntryRequest {
    pub title: String,
    pub kind: String,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntryRequest {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ListEntriesQuery {
    pub kind: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct QuickCaptureRequest {
    pub text: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub tags: Option<Vec<String>>,
    pub url: Option<String>,
}

#[derive(Debug)]
pub struct NewEntry {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub notes: String,
    pub url: Option<String>,
    pub source: String,
    pub tags_json: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

// -- Telegram types --

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub message: Option<TelegramMessage>,
    pub edited_message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub text: Option<String>,
    pub caption: Option<String>,
    pub chat: TelegramChat,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

// -- Response types --

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub database: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AcceptedResponse {
    pub status: &'static str,
    pub entry_id: String,
}
