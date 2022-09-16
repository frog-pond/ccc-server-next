use core::ops::DerefMut;

use axum::{
	extract::Path,
	response::{IntoResponse, Response},
	Json,
};
use http::StatusCode;
use tracing::instrument;

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

	#[error("error while processing proxied response from bonapp ({0})")]
	Response(reqwest::Error),

	#[error("unknown cafe")]
	UnknownCafe,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum ProxyRequestQueryParameters {
	CafeQuery { cafe: String },
	MenuQuery { cafe: String },
	ItemNutritionQuery { item: String },
}

#[test]
fn query_parameters_serialize() {
	let query = ProxyRequestQueryParameters::CafeQuery {
		cafe: "foo".to_string(),
	};
	assert_eq!(
		serde_urlencoded::to_string(query).ok(),
		Some("cafe=foo".to_string())
	);
}

#[axum_macros::debug_handler]
#[instrument(skip_all)]
pub async fn named_cafe_handler(
	Path((cafe_name,)): Path<(String,)>,
) -> Result<Json<types::food::BonAppCafeResponse>, BonAppProxyError> {
	if CAFE_NAME_MAP.contains_key(&cafe_name) {
		cafe(Path(CAFE_NAME_MAP.get(&cafe_name).unwrap().to_string())).await
	} else {
		tracing::warn!(?cafe_name, "unknown named cafe");
		Err(BonAppProxyError::UnknownCafe)
	}
}

#[axum_macros::debug_handler]
#[instrument(skip_all)]
pub async fn named_cafe_menu_handler(
	Path((cafe_name,)): Path<(String,)>,
) -> Result<Json<types::food::BonAppMenuResponse>, BonAppProxyError> {
	if CAFE_NAME_MAP.contains_key(&cafe_name) {
		cafe_menu(Path(CAFE_NAME_MAP.get(&cafe_name).unwrap().to_string())).await
	} else {
		tracing::warn!(?cafe_name, "unknown named cafe");
		Err(BonAppProxyError::UnknownCafe)
	}
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
	response
		.json()
		.await
		.map(Json)
		.map_err(BonAppProxyError::Response)
}

const fn query_base_url(query_type: &QueryType) -> &str {
	use QueryType::*;
	match query_type {
		Cafe => "https://legacy.cafebonappetit.com/api/2/cafes",
		Menu => "https://legacy.cafebonappetit.com/api/2/menus",
		ItemNutrition => "https://legacy.cafebonappetit.com/api/2/items",
	}
}

fn query_url(
	query_type: &QueryType,
	base_url: &str,
	id: String,
) -> Result<String, BonAppProxyError> {
	use {ProxyRequestQueryParameters::*, QueryType::*};
	let params = match query_type {
		Cafe => CafeQuery { cafe: id },
		Menu => MenuQuery { cafe: id },
		ItemNutrition => ItemNutritionQuery { item: id },
	};
	let url = format!("{}?{}", base_url, serde_urlencoded::to_string(params)?);
	Ok(url)
}

#[instrument]
async fn proxied_query<T>(
	query_type: QueryType,
	entity_id: &str,
) -> Result<Json<T>, BonAppProxyError>
where
	T: serde::de::DeserializeOwned,
{
	tracing::debug!(entity_id, ?query_type, "handling proxied BonApp request");

	let url = query_base_url(&query_type);

	let url: String = query_url(&query_type, url, entity_id.to_string())?;
	tracing::debug!(url);

	let response = {
		let span = tracing::trace_span!("proxy request");
		let _entered = span.enter();
		reqwest::get(url).await.map_err(BonAppProxyError::Request)
	}?;

	let result = {
		let span = tracing::trace_span!("proxy response");
		let _entered = span.enter();

		parse_response::<T>(response).await
	};

	result
}

#[instrument]
pub async fn cafe(
	Path(cafe_id): Path<String>,
) -> Result<Json<types::food::BonAppCafeResponse>, BonAppProxyError> {
	proxied_query::<types::food::BonAppCafesResponse>(QueryType::Cafe, &cafe_id)
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
pub async fn cafe_menu(
	Path(cafe_id): Path<String>,
) -> Result<Json<types::food::BonAppMenuResponse>, BonAppProxyError> {
	proxied_query(QueryType::Menu, &cafe_id).await
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
