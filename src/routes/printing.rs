use crate::color_printers_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    #[serde(rename = "colorPrinters")]
    color_printers: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: ResponseItem,
}

pub(crate) fn router() -> Router {
    Router::new().route("/color-printers", get(color_printers_handler))
}
