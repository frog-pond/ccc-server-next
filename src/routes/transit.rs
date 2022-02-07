use crate::transit_bus_handler;
use crate::transit_modes_handler;
use axum::{routing::get, Router};
use serde_json::value::Value;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModesItem {
    name: String,
    category: String,
    description: String,
    synopsis: String,
    url: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModesResponse {
    data: Vec<ModesItem>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BusTimesResponseItem {
    line: String,
    colors: Colors,
    notice: Option<String>,
    schedules: Vec<Schedule>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Colors {
    bar: String,
    dot: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Coordinate {
    // todo: add this type
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    days: Vec<String>,
    // todo: add this type
    coordinates: Option<Coordinate>,
    stops: Vec<String>,
    // todo: scope Value to String | Bool
    times: Vec<Vec<Value>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BusTimesResponse {
    data: Vec<BusTimesResponseItem>,
}

pub(crate) fn router() -> Router {
    let transit_routes = Router::new()
        .route("/bus", get(transit_bus_handler))
        .route("/modes", get(transit_modes_handler));

    Router::new().nest("/transit", transit_routes)
}
