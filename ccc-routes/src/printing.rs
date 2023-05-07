use axum::{routing::get, Router};
use ccc_handlers::github::color_printers_handler;

pub fn router() -> Router {
	Router::new().route("/color-printers", get(color_printers_handler))
}
