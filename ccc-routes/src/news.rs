use axum::{routing::get, Router};
use ccc_handlers::news::wp_json_handler;

pub fn router() -> Router {
	Router::new().route("/wpjson", get(wp_json_handler))
}
