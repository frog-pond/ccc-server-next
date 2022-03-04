use axum::{routing::get, Router};
use handlers::github::dictionary_handler;

pub fn router() -> Router {
    Router::new().route("/", get(dictionary_handler))
}
