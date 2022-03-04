use axum::{routing::get, Router};
use handlers::github::help_handler;

pub fn router() -> Router {
    Router::new().route("/help", get(help_handler))
}
