use axum::{routing::get, Router};
use ccc_handlers::reports::stav_mealtime_report_handler;

pub fn router() -> Router {
	Router::new().route("/stav", get(stav_mealtime_report_handler))
}
