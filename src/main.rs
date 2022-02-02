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

async fn gh_pages_handler(filename: &str) -> Result<Json<Value>, StatusCode> {
    let url = format!("https://stodevx.github.io/AAO-React-Native/{}", filename).to_string();
    let resp = request_handler(&url).await?;
    Ok(resp)
}

macro_rules! gh_pages_handler {
    ($name:ident,$filename:literal) => {
        async fn $name(_version: Version) -> Result<Json<Value>, StatusCode> {
            let data = gh_pages_handler($filename).await.unwrap();
            Ok(data)
        }
    };
}

macro_rules! gh_pages_handlers {
    ($([$name:ident, $filename:literal]),+ $(,)?) => {
        $(
            gh_pages_handler!($name, $filename);
        )+
    };
}

gh_pages_handlers!(
    [contacts_handler, "contact-info.json"],
    [dictionary_handler, "dictionary.json"],
    [faqs_handler, "faqs.json"],
    [pause_menu_handler, "pause-menu.json"],
    [color_printers_handler, "color-printers.json"],
    [hours_handler, "building-hours.json"],
    [help_handler, "help.json"],
    [transit_bus_handler, "bus-times.json"],
    [transit_modes_handler, "transportation.json"],
    [webcams_handler, "webcams.json"],
);

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
