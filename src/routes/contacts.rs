use crate::contacts_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    title: String,
    #[serde(rename = "camelCase")]
    phone_number: Option<String>,
    #[serde(rename = "camelCase")]
    button_text: Option<String>,
    category: String,
    image: Option<String>,
    synopsis: String,
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/contacts", get(contacts_handler))
}
