use axum::response::Json;
use reqwest::Method;
use serde::de::DeserializeOwned;

async fn gh_pages_handler<T>(filename: &str) -> Result<Json<T>, ccc_proxy::ProxyError>
where
	T: DeserializeOwned,
{
	let url = format!("https://stodevx.github.io/AAO-React-Native/{filename}").to_string();

	let request = ccc_proxy::global_proxy()
		.client()
		.request(Method::GET, url)
		.build()
		.map_err(ccc_proxy::ProxyError::ProxiedRequest)?;

	ccc_proxy::global_proxy()
		.send_request_parse_json::<T>(request)
		.await
		.map(Json)
}

macro_rules! gh_pages_handler {
	($name:ident,$filename:literal,$response_type:ty) => {
		/// # Errors
		///
		/// Will return `JsonProxyError` if the network request or json serialization failed
		pub async fn $name() -> Result<Json<$response_type>, ccc_proxy::ProxyError> {
			let data = gh_pages_handler::<$response_type>($filename).await?;
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
		ccc_types::contacts::Response
	],
	[
		dictionary_handler,
		"dictionary.json",
		ccc_types::dictionary::Response
	],
	[faqs_handler, "faqs.json", ccc_types::faqs::Response],
	[
		color_printers_handler,
		"color-printers.json",
		ccc_types::printing::Response
	],
	[
		pause_menu_handler,
		"pause-menu.json",
		ccc_types::food::PauseMenuResponse
	],
	[
		hours_handler,
		"building-hours.json",
		ccc_types::spaces::HoursResponse
	],
	[help_handler, "help.json", ccc_types::tools::Response],
	[
		transit_bus_handler,
		"bus-times.json",
		ccc_types::transit::BusTimesResponse
	],
	[
		transit_modes_handler,
		"transportation.json",
		ccc_types::transit::ModesResponse
	],
	[
		webcams_handler,
		"webcams.json",
		ccc_types::webcams::Response
	],
);
