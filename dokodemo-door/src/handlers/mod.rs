pub mod category;
pub mod entry;
pub mod tag;

use axum::Json;

use crate::db::Database;
use crate::models::HealthResponse;
use crate::AppState;

pub async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<HealthResponse> {
    let database = match state.db {
        Database::Postgres(_) => "postgres",
        Database::Sqlite(_) => "sqlite",
    };

    Json(HealthResponse {
        status: "ok",
        database,
    })
}
