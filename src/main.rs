use axum::{
    async_trait,
    extract::{FromRequest, Path, RequestParts},
    handler::Handler,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router, Server,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

#[derive(Debug)]
enum Version {
    V1,
}

#[derive(Debug)]
struct JsonProxyError(reqwest::Error);

#[tokio::main]
async fn main() {
    let meta_routes = Router::new()
        .route("/", get(root_handler))
        .route("/ping", get(heartbeat_handler));

    let api_routes = Router::new()
        .nest("/contacts", routes::contacts::router())
        .nest("/dictionary", routes::dictionary::router())
        .nest("/faqs", routes::faqs::router())
        .nest("/food", routes::food::router())
        .nest("/printing", routes::printing::router())
        .nest("/spaces", routes::spaces::router())
        .nest("/tools", routes::tools::router())
        .nest("/transit", routes::transit::router())
        .nest("/webcams", routes::webcams::router());

    let app = Router::new()
        .nest("/", meta_routes)
        .nest("/api/:version", api_routes)
        .fallback(fallback.into_service());

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .expect("server failed to exit successfully");
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

async fn gh_pages_handler<T>(filename: &str) -> Result<Json<T>, JsonProxyError>
where
    T: DeserializeOwned,
{
    let url = format!("https://stodevx.github.io/AAO-React-Native/{}", filename).to_string();
    let resp = request_handler(&url).await?;
    Ok(resp)
}

macro_rules! gh_pages_handler {
    ($name:ident,$filename:literal,$response_type:ty) => {
        async fn $name(_version: Version) -> Result<Json<$response_type>, JsonProxyError> {
            let data = gh_pages_handler($filename).await?;
            Ok(data)
        }
    };
}

macro_rules! gh_pages_handlers {
    ($([$name:ident, $filename:literal $(, $response_type:ty)?]),+ $(,)?) => {
        $(
            gh_pages_handler!($name, $filename $(, $response_type)?);
        )+
    };
}

gh_pages_handlers!(
    [
        contacts_handler,
        "contact-info.json",
        routes::contacts::Response
    ],
    [
        dictionary_handler,
        "dictionary.json",
        routes::dictionary::Response
    ],
    [faqs_handler, "faqs.json", routes::faqs::Response],
    [
        color_printers_handler,
        "color-printers.json",
        routes::printing::Response
    ],
    [
        pause_menu_handler,
        "pause-menu.json",
        routes::food::PauseMenuResponse
    ],
    [
        hours_handler,
        "building-hours.json",
        routes::spaces::HoursResponse
    ],
    [help_handler, "help.json", routes::tools::Response],
    [
        transit_bus_handler,
        "bus-times.json",
        routes::transit::BusTimesResponse
    ],
    [
        transit_modes_handler,
        "transportation.json",
        routes::transit::ModesResponse
    ],
    [webcams_handler, "webcams.json", routes::webcams::Response],
);

async fn request_handler<T>(path: &str) -> Result<Json<T>, JsonProxyError>
where
    T: DeserializeOwned,
{
    let response = reqwest::get(path).await.map_err(JsonProxyError)?;

    response.json().await.map(Json).map_err(JsonProxyError)
}

impl IntoResponse for JsonProxyError {
    fn into_response(self) -> axum::response::Response {
        // `self.0` is a `reqwest::Error`.  Just format it as a string.
        let body = axum::body::boxed(axum::body::Full::from(self.0.to_string()));

        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body)
            .unwrap()
    }
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
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Missing Version").into_response())?;

        match version.as_str() {
            "v1" => Ok(Version::V1),
            _ => Err((StatusCode::NOT_FOUND, "Unknown Version").into_response()),
        }
    }
}

mod routes;
