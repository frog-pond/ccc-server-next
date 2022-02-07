use crate::help_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Button {
    title: String,
    action: String,
    params: Params,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Params {
    url: Option<String>,
    number: Option<String>,
    to: Option<String>,
    subject: Option<String>,
    body: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    key: String,
    enabled: bool,
    hidden: bool,
    title: String,
    body: String,
    buttons: Vec<Button>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/help", get(help_handler))
}
