use axum::{
	extract::Path,
	response::{IntoResponse, Response},
	Json,
};
use http::StatusCode;
use tracing::instrument;

const BONAPP_CAFE_KEY_STAV: &str = "stav-hall";
const BONAPP_CAFE_KEY_CAGE: &str = "the-cage";
const BONAPP_CAFE_KEY_KINGS_ROOM: &str = "kings-room";
const BONAPP_CAFE_KEY_BURTON: &str = "burton";
const BONAPP_CAFE_KEY_LDC: &str = "ldc";
const BONAPP_CAFE_KEY_SAYLES: &str = "sayles";
const BONAPP_CAFE_KEY_WEITZ: &str = "weitz";

#[derive(Debug)]
enum NamedBonAppCafe {
	Stav,
	Cage,
	KingsRoom,
	Burton,
	Ldc,
	Sayles,
	Weitz,
}

impl NamedBonAppCafe {
	fn from_name(name: &str) -> Option<Self> {
		use NamedBonAppCafe::*;
		match name {
			BONAPP_CAFE_KEY_STAV => Some(Stav),
			BONAPP_CAFE_KEY_CAGE => Some(Cage),
			BONAPP_CAFE_KEY_KINGS_ROOM => Some(KingsRoom),
			BONAPP_CAFE_KEY_BURTON => Some(Burton),
			BONAPP_CAFE_KEY_LDC => Some(Ldc),
			BONAPP_CAFE_KEY_SAYLES => Some(Sayles),
			BONAPP_CAFE_KEY_WEITZ => Some(Weitz),
			_ => None,
		}
	}

	const fn get_bonapp_cafe_id(&self) -> u32 {
		use NamedBonAppCafe::*;
		match self {
			Stav => 261,
			Cage => 262,
			KingsRoom => 263,
			Burton => 35,
			Ldc => 36,
			Sayles => 24,
			Weitz => 458,
		}
	}
}

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
) -> Result<Json<types::food::BonAppCafesResponse>, BonAppProxyError> {
	if let Some(named_cafe) = NamedBonAppCafe::from_name(&cafe_name) {
		cafe(&named_cafe.get_bonapp_cafe_id().to_string()).await
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

#[instrument]
async fn proxied_query<T>(
	query_type: QueryType,
	entity_id: &str,
) -> Result<Json<T>, BonAppProxyError>
where
	T: serde::de::DeserializeOwned,
{
	tracing::debug!(entity_id, ?query_type, "handling proxied BonApp request");

	let url = match query_type {
		QueryType::Cafe => "https://legacy.cafebonappetit.com/api/2/cafes",
		QueryType::Menu => "https://legacy.cafebonappetit.com/api/2/menus",
		QueryType::ItemNutrition => "https://legacy.cafebonappetit.com/api/2/items",
	};

	let url: Result<String, BonAppProxyError> = {
		let params = match query_type {
			QueryType::Cafe => ProxyRequestQueryParameters::CafeQuery {
				cafe: entity_id.to_string(),
			},
			QueryType::Menu => ProxyRequestQueryParameters::MenuQuery {
				cafe: entity_id.to_string(),
			},
			QueryType::ItemNutrition => ProxyRequestQueryParameters::ItemNutritionQuery {
				item: entity_id.to_string(),
			},
		};
		let url = format!("{}?{}", url, serde_urlencoded::to_string(params)?);
		Ok(url)
	};

	let url = url?;
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
async fn cafe(cafe_id: &str) -> Result<Json<types::food::BonAppCafesResponse>, BonAppProxyError> {
	proxied_query(QueryType::Cafe, cafe_id).await
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
