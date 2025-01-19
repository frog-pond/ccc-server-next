use core::ops::DerefMut;

use axum::{
	extract::Path,
	response::{IntoResponse, Response},
	Json,
};
use http::StatusCode;
use reqwest::Method;
use tracing::instrument;

use ccc_proxy::ProxyError;

#[instrument]
async fn send_proxied_query<T>(
	query_type: QueryType,
	entity_id: &str,
) -> Result<Json<T>, BonAppProxyError>
where
	T: serde::de::DeserializeOwned,
{
	tracing::debug!(entity_id, ?query_type, "handling proxied BonApp request");

	let (base_url, entity) = get_query_base_url_and_entity(&query_type);

	let bon_app_auth =
		std::env::var("BON_APPETIT_AUTH").expect("BON_APPETIT_AUTH credential not set");
	let auth_header_value = format!("Basic {}", bon_app_auth);

	let request = ccc_proxy::global_proxy()
		.client()
		.request(Method::GET, base_url)
		.query(&[(entity, entity_id)])
		.header("Authorization", auth_header_value)
		.build()
		.map_err(ProxyError::ProxiedRequest)
		.map_err(BonAppProxyError::GenericProxy)?;

	ccc_proxy::global_proxy()
		.send_request_parse_json::<T>(request)
		.await
		.map(Json)
		.map_err(BonAppProxyError::GenericProxy)
}

/// Map which stores mappings of known cafe names to cafe IDs.
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
	#[error("unknown cafe")]
	UnknownCafe,

	#[error("error from generic proxy: {0}")]
	GenericProxy(ProxyError),
}

#[derive(Debug)]
enum QueryType {
	Cafe,
	Menu,
	ItemNutrition,
}

use QueryType::*;

#[inline]
const fn get_query_base_url_and_entity(query_type: &QueryType) -> (&str, &str) {
	match query_type {
		Cafe => (
			"https://cafemanager-api.cafebonappetit.com/api/2/cafes",
			"cafe",
		),
		Menu => (
			"https://cafemanager-api.cafebonappetit.com/api/2/menus",
			"cafe",
		),
		ItemNutrition => (
			"https://cafemanager-api.cafebonappetit.com/api/2/items",
			"item",
		),
	}
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
	send_proxied_query::<ccc_types::food::BonAppCafesResponse>(Cafe, &cafe_id)
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
	send_proxied_query::<ccc_types::food::BonAppMenuMultipleCafesResponse>(Menu, &cafe_id)
		.await
		.map(|Json(result)| Json(result.as_single_day_response(&cafe_id)))
}

#[instrument]
pub async fn nutrition_handler(
	Path(item_id): Path<String>,
) -> Result<Json<ccc_types::food::ItemNutritionResponse>, BonAppProxyError> {
	send_proxied_query(ItemNutrition, &item_id).await
}

impl IntoResponse for BonAppProxyError {
	fn into_response(self) -> axum::response::Response {
		let text = self.to_string();

		let body = text.into();

		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(body)
			.unwrap()
	}
}
