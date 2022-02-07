use crate::webcams_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    name: String,
    #[serde(rename = "pageUrl")]
    page_url: String,
    #[serde(rename = "streamUrl")]
    stream_url: String,
    thumbnail: String,
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
    tagline: String,
    #[serde(rename = "accentColor")]
    accent_color: Vec<i64>,
    #[serde(rename = "textColor")]
    text_color: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/webcams", get(webcams_handler))
}
