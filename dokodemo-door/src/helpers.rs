use crate::db::Database;
use crate::error::ApiError;

const ALLOWED_STATUSES: &[&str] = &["planned", "in_progress", "completed", "dropped"];

pub async fn validate_kind(db: &Database, kind: &str) -> Result<(), ApiError> {
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

pub fn validate_status(status: &str) -> Result<(), ApiError> {
    if ALLOWED_STATUSES.contains(&status) {
        return Ok(());
    }

    Err(ApiError::BadRequest(format!(
        "invalid status `{status}`, allowed: {}",
        ALLOWED_STATUSES.join(", ")
    )))
}

pub fn serialize_tags(tags: Vec<String>) -> Result<String, ApiError> {
    serde_json::to_string(&tags)
        .map_err(|err| ApiError::Internal(format!("failed to serialize tags: {err}")))
}

pub fn summarize_title(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or_default().trim();
    let mut title = first_line.chars().take(80).collect::<String>();
    if title.is_empty() {
        title = "quick note".to_string();
    }
    title
}

pub fn extract_url_from_text(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.starts_with("http://") || part.starts_with("https://"))
        .map(|value| {
            value
                .trim_end_matches(&[')', ']', '}', ',', '.', ';'][..])
                .to_string()
        })
}

pub fn map_unique_error(e: sqlx::Error, name: &str, entity: &str) -> ApiError {
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
