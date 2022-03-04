use axum::{routing::get, Router};
use handlers::github::transit_bus_handler;
use handlers::github::transit_modes_handler;

pub fn router() -> Router {
    Router::new()
        .route("/bus", get(transit_bus_handler))
        .route("/modes", get(transit_modes_handler))
}
