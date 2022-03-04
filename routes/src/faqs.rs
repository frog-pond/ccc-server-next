use axum::{routing::get, Router};
use handlers::github::faqs_handler;

pub fn router() -> Router {
    Router::new().route("/", get(faqs_handler))
}
