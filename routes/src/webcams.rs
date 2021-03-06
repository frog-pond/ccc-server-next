use axum::{routing::get, Router};
use handlers::github::webcams_handler;

#[must_use]
pub fn router() -> Router {
	Router::new().route("/", get(webcams_handler))
}
