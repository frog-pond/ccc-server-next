use axum::{routing::get, Router};
use ccc_handlers::github::webcams_handler;

pub fn router() -> Router {
	Router::new().route("/", get(webcams_handler))
}
