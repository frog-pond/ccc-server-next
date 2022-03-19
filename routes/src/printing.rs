use axum::{routing::get, Router};
use handlers::github::color_printers_handler;

#[must_use]
pub fn router() -> Router {
	Router::new().route("/color-printers", get(color_printers_handler))
}
