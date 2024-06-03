use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use axum::{routing::get, Router};

use dokodemo_door::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use tokio::signal;

#[derive(Serialize)]
struct Health {
    healthy: bool,
}

pub(crate) async fn health() -> impl IntoResponse {
    let health = Health { healthy: true };
    (StatusCode::OK, Json(health))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    opentelemetry::global::shutdown_tracer_provider();
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/").subscription_endpoint("/ws"),
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let listener = tokio::net::TcpListener::bind(socket).await.unwrap();

    let state = HashMap::<i32, String>::new();

    // For metrics, see https://github.com/oliverjumpertz/axum-graphql/blob/main/src/main.rs
    log::info!("starting server");
    let app = Router::new()
        .route("/health", get(health))
        .route("/", get(graphql_playground))
        // .route("/graphql", post(graphql_handler))
        // .layer(Extension(schema))
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
