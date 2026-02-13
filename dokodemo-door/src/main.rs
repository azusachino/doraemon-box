use anyhow::{Context, Result};
use axum::{routing::get, Router};
use dokodemo_door::db::{connect_database, run_migrations};
use dokodemo_door::handlers::health;
use dokodemo_door::routes::api_routes;
use dokodemo_door::AppState;
use std::{env, net::SocketAddr};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
