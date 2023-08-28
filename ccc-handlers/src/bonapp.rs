use core::ops::DerefMut;
use std::sync::OnceLock;

use axum::{
	extract::Path,
	response::{IntoResponse, Response},
	Json,
};
use http::StatusCode;
use tracing::instrument;

struct BonAppProxy {
	client: Option<reqwest::Client>,
}

static GLOBAL_BONAPP_PROXY: OnceLock<BonAppProxy> = OnceLock::new();

impl BonAppProxy {
	fn handle() -> &'static BonAppProxy {
		GLOBAL_BONAPP_PROXY.get_or_init(|| {
			let client = reqwest::Client::builder()
				.user_agent("ccc-server-next/0.0.0")
				.build()
				.unwrap();

			BonAppProxy {
				client: Some(client),
			}
		})
	}
}

impl BonAppProxy {
	#[instrument(skip(self))]
	async fn send_proxied_query<T>(
		&self,
		query_type: QueryType,
		entity_id: &str,
	) -> Result<Json<T>, BonAppProxyError>
	where
		T: serde::de::DeserializeOwned,
	{
		tracing::debug!(entity_id, ?query_type, "handling proxied BonApp request");

		let base_url = get_query_base_url(&query_type);
		let id = entity_id.to_string();

		let url: String = build_query_url(&query_type, base_url, id)?;

		tracing::debug!(?url, "sending proxied request");

		let response = match &self.client {
			Some(client) => {
				client
					.execute(client.get(url).build().map_err(BonAppProxyError::Request)?)
					.await
			}
			None => {
				tracing::warn!(
					"shared client was not initialized; falling back on inefficient request handler!"
				);
				reqwest::get(url).await
			}
		};

		// let response = reqwest::get(url).await;

		tracing::debug!(?response, "got response");

		let response = response.map_err(BonAppProxyError::Request)?;

		parse_response::<T>(response).await
	}
}

const CAFE_NAME_MAP: phf::Map<&str, u32> = phf::phf_map! {
	"stav-hall" => 261,
	"the-cage" => 262,
	"kings-room" => 263,
	"burton" => 35,
	"ldc" => 36,
	"sayles" => 24,
	"weitz" => 458,
};

#[derive(thiserror::Error, Debug)]
pub enum BonAppProxyError {
	#[error("error while encoding query string")]
	QueryStringEncoding(#[from] serde_urlencoded::ser::Error),

	#[error("error while sending proxied request to bonapp")]
	Request(reqwest::Error),

	#[error("error while receiving proxied response from bonapp ({0})")]
	ResponseAcquisition(reqwest::Error),

	#[error("error while parsing proxied response from bonapp ({0}):\n{1}")]
	ResponseParse(serde_json::Error, String),

	#[error("unknown cafe")]
	UnknownCafe,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum ProxyRequestQueryParameters {
	Cafe { cafe: String },
	Menu { cafe: String },
	ItemNutrition { item: String },
}

#[test]
fn query_parameters_serialize() {
	let query = ProxyRequestQueryParameters::Cafe {
		cafe: "foo".to_string(),
	};
	assert_eq!(
		serde_urlencoded::to_string(query).ok(),
		Some("cafe=foo".to_string())
	);
}

#[derive(Debug)]
enum QueryType {
	Cafe,
	Menu,
	ItemNutrition,
}

#[instrument]
async fn parse_response<T>(response: reqwest::Response) -> Result<Json<T>, BonAppProxyError>
where
	T: serde::de::DeserializeOwned,
{
	let response = response
		.text()
		.await
		.map_err(BonAppProxyError::ResponseAcquisition)?;

	let json = serde_json::from_str(&response);

	match json {
		Ok(value) => Ok(Json(value)),
		Err(e) => Err(BonAppProxyError::ResponseParse(e, response)),
	}
}

#[inline]
const fn get_query_base_url(query_type: &QueryType) -> &str {
	use QueryType::*;
	match query_type {
		Cafe => "https://legacy.cafebonappetit.com/api/2/cafes",
		Menu => "https://legacy.cafebonappetit.com/api/2/menus",
		ItemNutrition => "https://legacy.cafebonappetit.com/api/2/items",
	}
}

fn build_query_url(
	query_type: &QueryType,
	base_url: &str,
	id: String,
) -> Result<String, BonAppProxyError> {
	let params = match query_type {
		QueryType::Cafe => ProxyRequestQueryParameters::Cafe { cafe: id },
		QueryType::Menu => ProxyRequestQueryParameters::Menu { cafe: id },
		QueryType::ItemNutrition => ProxyRequestQueryParameters::ItemNutrition { item: id },
	};
	let url = format!("{}?{}", base_url, serde_urlencoded::to_string(params)?);
	Ok(url)
}

#[instrument(skip_all)]
pub async fn named_cafe_handler(
	Path((cafe_name,)): Path<(String,)>,
) -> Result<Json<ccc_types::food::BonAppCafeResponse>, BonAppProxyError> {
	if let Some(id) = CAFE_NAME_MAP.get(&cafe_name) {
		cafe_handler(Path(id.to_string())).await
	} else {
		tracing::warn!(?cafe_name, "unknown named cafe");
		Err(BonAppProxyError::UnknownCafe)
	}
}

#[instrument(skip_all)]
pub async fn named_cafe_menu_handler(
	Path((cafe_name,)): Path<(String,)>,
) -> Result<Json<ccc_types::food::BonAppMenuSingleCafeResponse>, BonAppProxyError> {
	if let Some(id) = CAFE_NAME_MAP.get(&cafe_name) {
		cafe_menu_handler(Path(id.to_string())).await
	} else {
		tracing::warn!(?cafe_name, "unknown named cafe");
		Err(BonAppProxyError::UnknownCafe)
	}
}

#[instrument]
pub async fn cafe_handler(
	Path(cafe_id): Path<String>,
) -> Result<Json<ccc_types::food::BonAppCafeResponse>, BonAppProxyError> {
	BonAppProxy::handle()
		.send_proxied_query::<ccc_types::food::BonAppCafesResponse>(QueryType::Cafe, &cafe_id)
		.await
		.map(|mut result| {
			let cafes = result.deref_mut().cafes_mut();

			// TODO: This might be a bit less descriptive than it should be.
			// What _really_ happened is we got back a response that didn't have
			// a top level key corresponding to the cafe id.  Not that the cafe is
			// unknown; rather it is known, we got a response back, but it didn't
			// look like it was supposed to.
			cafes
				.remove(&cafe_id)
				.map(Json)
				.expect("cafe did not appear in the response")
		})
}

#[instrument]
pub async fn cafe_menu_handler(
	Path(cafe_id): Path<String>,
) -> Result<Json<ccc_types::food::BonAppMenuSingleCafeResponse>, BonAppProxyError> {
	BonAppProxy::handle()
		.send_proxied_query::<ccc_types::food::BonAppMenuMultipleCafesResponse>(
			QueryType::Menu,
			&cafe_id,
		)
		.await
		.map(|Json(result)| Json(result.as_single_day_response(&cafe_id)))
}

#[instrument]
pub async fn nutrition_handler(
	Path(item_id): Path<String>,
) -> Result<Json<ccc_types::food::ItemNutritionResponse>, BonAppProxyError> {
	BonAppProxy::handle()
		.send_proxied_query(QueryType::ItemNutrition, &item_id)
		.await
}

impl IntoResponse for BonAppProxyError {
	fn into_response(self) -> axum::response::Response {
		let text = self.to_string();

		let body = axum::body::boxed(axum::body::Full::from(text));

		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(body)
			.unwrap()
	}
}
