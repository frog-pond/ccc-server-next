use crate::dictionary_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    word: String,
    definition: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/dictionary", get(dictionary_handler))
}
