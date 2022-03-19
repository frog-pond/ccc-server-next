use axum::{
	http::Response,
	response::{IntoResponse, Json},
};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

#[derive(thiserror::Error, Debug)]
pub enum JsonProxyError {
	#[error("error during proxied request")]
	Request(#[from] reqwest::Error),
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
		/// # Errors
		///
		/// Will return `JsonProxyError` if the network request or json serialization failed
		pub async fn $name() -> Result<Json<$response_type>, JsonProxyError> {
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
		types::contacts::Response
	],
	[
		dictionary_handler,
		"dictionary.json",
		types::dictionary::Response
	],
	[faqs_handler, "faqs.json", types::faqs::Response],
	[
		color_printers_handler,
		"color-printers.json",
		types::printing::Response
	],
	[
		pause_menu_handler,
		"pause-menu.json",
		types::food::PauseMenuResponse
	],
	[
		hours_handler,
		"building-hours.json",
		types::spaces::HoursResponse
	],
	[help_handler, "help.json", types::tools::Response],
	[
		transit_bus_handler,
		"bus-times.json",
		types::transit::BusTimesResponse
	],
	[
		transit_modes_handler,
		"transportation.json",
		types::transit::ModesResponse
	],
	[webcams_handler, "webcams.json", types::webcams::Response],
);

async fn request_handler<T>(path: &str) -> Result<Json<T>, JsonProxyError>
where
	T: DeserializeOwned,
{
	let response = reqwest::get(path).await.map_err(JsonProxyError::Request)?;

	response
		.json()
		.await
		.map(Json)
		.map_err(JsonProxyError::Request)
}

impl IntoResponse for JsonProxyError {
	fn into_response(self) -> axum::response::Response {
		// [this is a stub, where `self` is JsonProxyError]
		let text = match self {
			Self::Request(e) => e.to_string(),
		};

		let body = axum::body::boxed(axum::body::Full::from(text));

		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(body)
			.unwrap()
	}
}
