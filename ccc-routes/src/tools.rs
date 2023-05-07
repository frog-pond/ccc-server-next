use axum::{routing::get, Router};
use ccc_handlers::github::help_handler;

pub fn router() -> Router {
	Router::new().route("/help", get(help_handler))
}
