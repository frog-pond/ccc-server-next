use crate::faqs_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FAQResponse {
    text: String,
}

pub(crate) fn router() -> Router {
    Router::new().route("/faqs", get(faqs_handler))
}
