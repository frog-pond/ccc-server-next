use axum::{routing::get, Router};
use ccc_handlers::github::faqs_handler;

#[must_use]
pub fn router() -> Router {
	Router::new().route("/", get(faqs_handler))
}
