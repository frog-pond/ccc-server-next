use axum::{routing::get, Router};
use ccc_handlers::github::help_handler;

#[must_use]
pub fn router() -> Router {
	Router::new().route("/help", get(help_handler))
}
