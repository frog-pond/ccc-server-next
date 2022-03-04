use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "WebcamsResponseItem")]
pub struct ResponseItem {
    name: String,
    page_url: String,
    stream_url: String,
    thumbnail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail_url: Option<String>,
    tagline: String,
    accent_color: [i64; 3],
    text_color: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "WebcamsResponse")]
pub struct Response {
    data: Vec<ResponseItem>,
}
