use axum::{routing::get, Router};
use ccc_upstream_handlers::github::contacts_handler;

pub fn router() -> Router {
	Router::new().route("/", get(contacts_handler))
}
