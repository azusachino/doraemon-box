pub mod auth;
pub mod db;
pub mod error;
pub mod handlers;
pub mod helpers;
pub mod models;
pub mod routes;

use db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub telegram_webhook_secret: Option<String>,
    pub api_key: Option<String>,
}
