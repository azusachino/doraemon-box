use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, patch, post},
    Router,
};

use crate::auth::api_key_auth;
use crate::handlers::{category, entry, tag};
use crate::AppState;

pub fn api_routes(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route(
            "/entries",
            post(entry::create_entry).get(entry::list_entries),
        )
        .route(
            "/entries/:id",
            get(entry::get_entry)
                .patch(entry::update_entry)
                .delete(entry::delete_entry),
        )
        .route("/quick-capture", post(entry::quick_capture))
        .route(
            "/categories",
            post(category::create_category).get(category::list_categories),
        )
        .route(
            "/categories/:id",
            patch(category::update_category).delete(category::delete_category),
        )
        .route("/tags", post(tag::create_tag).get(tag::list_tags))
        .route("/tags/:id", delete(tag::delete_tag))
        .layer(from_fn_with_state(state, api_key_auth));

    Router::new().merge(protected_routes).route(
        "/integrations/telegram/update",
        post(entry::telegram_capture),
    )
}
