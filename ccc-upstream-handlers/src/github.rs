use axum::response::Json;
use reqwest::Method;
use serde::de::DeserializeOwned;

async fn gh_pages_handler<T>(
	filename: &str,
	repo: &str,
) -> Result<Json<T>, ccc_upstream_proxy::ProxyError>
where
	T: DeserializeOwned,
{
	let url = format!("https://stodevx.github.io/{repo}/{filename}");

	let request = ccc_upstream_proxy::global_proxy()
		.client()
		.request(Method::GET, url)
		.build()
		.map_err(ccc_upstream_proxy::ProxyError::ProxiedRequest)?;

	ccc_upstream_proxy::global_proxy()
		.send_request_parse_json::<T>(request)
		.await
		.map(Json)
}

macro_rules! gh_pages_handler {
	($name:ident,$repo:literal,$filename:literal,$response_type:ty) => {
		/// # Errors
		///
		/// Will return `JsonProxyError` if the network request or json serialization failed
		pub async fn $name() -> Result<Json<$response_type>, ccc_upstream_proxy::ProxyError> {
			let data = gh_pages_handler::<$response_type>($filename, $repo).await?;
			Ok(data)
		}
	};
}

macro_rules! gh_pages_handlers {
    ($([$name:ident, $repo:literal, $filename:literal, $response_type:ty]),+ $(,)?) => {
        $(
            gh_pages_handler!($name, $repo, $filename, $response_type);
        )+
    };
}

gh_pages_handlers!(
	[
		contacts_handler,
		"AAO-React-Native",
		"contact-info.json",
		ccc_types::contacts::Response
	],
	[
		dictionary_handler,
		"AAO-React-Native",
		"dictionary.json",
		ccc_types::dictionary::Response
	],
	[
		faqs_handler,
		"AAO-React-Native",
		"faqs.json",
		ccc_types::faqs::Response
	],
	[
		color_printers_handler,
		"AAO-React-Native",
		"color-printers.json",
		ccc_types::printing::Response
	],
	[
		pause_menu_handler,
		"AAO-React-Native",
		"pause-menu.json",
		ccc_types::food::PauseMenuResponse
	],
	[
		hours_handler,
		"AAO-React-Native",
		"building-hours.json",
		ccc_types::spaces::HoursResponse
	],
	[
		help_handler,
		"AAO-React-Native",
		"help.json",
		ccc_types::tools::Response
	],
	[
		transit_bus_handler,
		"AAO-React-Native",
		"bus-times.json",
		ccc_types::transit::BusTimesResponse
	],
	[
		transit_modes_handler,
		"AAO-React-Native",
		"transportation.json",
		ccc_types::transit::ModesResponse
	],
	[
		webcams_handler,
		"AAO-React-Native",
		"webcams.json",
		ccc_types::webcams::Response
	],
	[
		stav_mealtime_handler,
		"stav-mealtimes",
		"two-weeks.json",
		serde_json::Value
	],
);
