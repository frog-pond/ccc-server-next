use axum::{routing::get, Router};
use ccc_upstream_handlers::github::faqs_handler;

pub fn router() -> Router {
	Router::new().route("/", get(faqs_handler))
}
