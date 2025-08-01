use axum::{routing::get, Router};
use ccc_upstream_handlers::github::dictionary_handler;

pub fn router() -> Router {
	Router::new().route("/", get(dictionary_handler))
}
