use crate::contacts_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    title: String,
    #[serde(rename = "phoneNumber")]
    phone_number: Option<String>,
    #[serde(rename = "buttonText")]
    button_text: Option<String>,
    category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    synopsis: String,
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/", get(contacts_handler))
}
