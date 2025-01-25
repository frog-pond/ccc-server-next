use axum::{routing::get, Router};
use ccc_handlers::streams::{archived_handler, upcoming_handler};

pub fn router() -> Router {
	Router::new()
		.route("/archived", get(archived_handler))
		.route("/upcoming", get(upcoming_handler))
}
