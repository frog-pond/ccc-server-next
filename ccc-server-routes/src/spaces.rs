use axum::{routing::get, Router};
use ccc_upstream_handlers::github::hours_handler;

pub fn router() -> Router {
	Router::new().route("/hours", get(hours_handler))
}
