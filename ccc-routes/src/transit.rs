use axum::{routing::get, Router};
use ccc_handlers::github::{transit_bus_handler, transit_modes_handler};

pub fn router() -> Router {
	Router::new()
		.route("/bus", get(transit_bus_handler))
		.route("/modes", get(transit_modes_handler))
}
