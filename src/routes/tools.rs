use crate::help_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Button {
    pub title: String,
    pub action: String,
    pub params: Params,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Params {
    pub url: Option<String>,
    pub number: Option<String>,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    pub key: String,
    pub enabled: bool,
    pub hidden: bool,
    pub title: String,
    pub body: String,
    pub buttons: Vec<Button>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub data: Vec<ResponseItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/help", get(help_handler))
}
