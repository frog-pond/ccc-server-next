use axum::{routing::get, Router};
use handlers::github::hours_handler;

#[must_use]
pub fn router() -> Router {
	Router::new().route("/hours", get(hours_handler))
}
