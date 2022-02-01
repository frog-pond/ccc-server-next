use axum::{
    async_trait,
    extract::{FromRequest, Path, RequestParts},
    handler::Handler,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router, Server,
};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
enum Version {
    V1,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/ping", get(heartbeat_handler))
        .route("/:version/contacts", get(contacts_handler))
        .route("/:version/dictionary", get(dictionary_handler))
        .route("/:version/faqs", get(faqs_handler))
        .route("/:version/food/named/menu/pause", get(pause_menu_handler))
        .route(
            "/:version/printing/color-printers",
            get(color_printers_handler),
        )
        .route("/:version/spaces/hours", get(hours_handler))
        .route("/:version/tools/help", get(help_handler))
        .route("/:version/transit/bus", get(transit_bus_handler))
        .route("/:version/transit/modes", get(transit_modes_handler))
        .route("/:version/webcams", get(webcams_handler))
        .fallback(fallback.into_service());

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[allow(clippy::unused_async)]
async fn root_handler() -> Result<&'static str, StatusCode> {
    Ok("Hello world!")
}

#[allow(clippy::unused_async)]
async fn heartbeat_handler() -> Result<&'static str, StatusCode> {
    Ok("pong")
}

#[allow(clippy::unused_async)]
async fn fallback() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found".to_string())
}

async fn contacts_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("contact-info.json").await.unwrap();
    Ok(data)
}

async fn dictionary_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("dictionary.json").await.unwrap();
    Ok(data)
}

async fn faqs_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("faqs.json").await.unwrap();
    Ok(data)
}

async fn pause_menu_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("pause-menu.json").await.unwrap();
    Ok(data)
}

async fn color_printers_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("color-printers.json").await.unwrap();
    Ok(data)
}

async fn hours_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("building-hours.json").await.unwrap();
    Ok(data)
}

async fn help_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("help.json").await.unwrap();
    Ok(data)
}

async fn transit_bus_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("bus-times.json").await.unwrap();
    Ok(data)
}

async fn transit_modes_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("transportation.json").await.unwrap();
    Ok(data)
}

async fn webcams_handler(_version: Version) -> Result<Json<Value>, StatusCode> {
    let data = gh_pages_handler("webcams.json").await.unwrap();
    Ok(data)
}

async fn gh_pages_handler(filename: &str) -> Result<Json<Value>, StatusCode> {
    let url = format!("https://stodevx.github.io/AAO-React-Native/{}", filename).to_string();
    let resp = request_handler(&url).await;
    Ok(resp.unwrap())
}

async fn request_handler(path: &str) -> Result<Json<Value>, StatusCode> {
    let result = reqwest::get(path).await;

    if result.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let response = result.unwrap().json().await.unwrap();
    Ok(Json(response))
}

#[async_trait]
impl<B> FromRequest<B> for Version
where
    B: Send,
{
    type Rejection = Response;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let params = Path::<HashMap<String, String>>::from_request(req)
            .await
            .map_err(IntoResponse::into_response)?;

        let version = params
            .get("version")
            .ok_or_else(|| (StatusCode::NOT_FOUND, "missing version").into_response())?;

        match version.as_str() {
            "v1" => Ok(Version::V1),
            _ => Err((StatusCode::NOT_FOUND, "Unknown Version").into_response()),
        }
    }
}
