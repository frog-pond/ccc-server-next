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
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail_url: Option<String>,
    tagline: String,
    #[serde(rename = "accentColor")]
    accent_color: [i64; 3],
    #[serde(rename = "textColor")]
    text_color: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/", get(webcams_handler))
}
