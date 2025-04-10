use axum::{
	extract::Query,
	response::{IntoResponse, Json, Response},
};
use chrono::{Duration, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Deserialize)]
pub struct StreamParams {
	#[serde(default)]
	date_from: String,
	#[serde(default)]
	date_to: String,
	#[serde(default = "default_sort")]
	sort: String,
}

fn default_sort() -> String {
	"ascending".to_string()
}

#[derive(Debug, Serialize)]
enum QueryClass {
	Archived,
	Upcoming,
}

impl std::fmt::Display for QueryClass {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			QueryClass::Archived => write!(f, "archived"),
			QueryClass::Upcoming => write!(f, "current"),
		}
	}
}

#[inline]
const fn get_query_base_url_and_entity(query_class: &QueryClass) -> (&str, &str) {
	match query_class {
		QueryClass::Archived => (
			"https://www.stolaf.edu/multimedia/api/collection",
			"archived",
		),
		QueryClass::Upcoming => (
			"https://www.stolaf.edu/multimedia/api/collection",
			"current",
		),
	}
}

#[instrument]
async fn send_proxied_query<T>(
	query_class: QueryClass,
	date_from: &str,
	date_to: &str,
	sort: &str,
) -> Result<Json<T>, StreamProxyError>
where
	T: serde::de::DeserializeOwned,
{
	tracing::debug!(
		sort,
		date_to,
		date_from,
		?query_class,
		"handling proxied Stream request"
	);

	let (base_url, _class) = get_query_base_url_and_entity(&query_class);
	let query_type = query_class.to_string();

	let request = ccc_proxy::global_proxy()
		.client()
		.get(base_url)
		.query(&[
			("class", query_type.as_str()),
			("date_from", date_from),
			("date_to", date_to),
			("sort", sort),
		])
		.build()
		.map_err(ccc_proxy::ProxyError::ProxiedRequest)
		.map_err(StreamProxyError::GenericProxy)?;

	ccc_proxy::global_proxy()
		.send_request_parse_json::<T>(request)
		.await
		.map(Json)
		.map_err(StreamProxyError::GenericProxy)
}

#[instrument(skip(date_from_fn, date_to_fn))]
async fn handle_stream_request(
	params: StreamParams,
	date_from_fn: impl Fn(chrono::DateTime<Utc>) -> chrono::DateTime<Utc>,
	date_to_fn: impl Fn(chrono::DateTime<Utc>) -> chrono::DateTime<Utc>,
	query_class: QueryClass,
) -> Result<Json<ccc_types::streams::StreamResponse>, StreamProxyError> {
	let now = Utc::now();

	let date_from = if !params.date_from.is_empty() {
		params.date_from.clone()
	} else {
		date_from_fn(now).format("%Y-%m-%d").to_string()
	};

	let date_to = if !params.date_to.is_empty() {
		params.date_to.clone()
	} else {
		date_to_fn(now).format("%Y-%m-%d").to_string()
	};

	send_proxied_query(query_class, &date_from, &date_to, &params.sort).await
}

#[instrument]
pub async fn upcoming_handler(
	Query(params): Query<StreamParams>,
) -> Result<Json<ccc_types::streams::StreamResponse>, StreamProxyError> {
	handle_stream_request(
		params,
		|now| now,
		|now| now + Duration::days(60),
		QueryClass::Upcoming,
	)
	.await
}

#[instrument]
pub async fn archived_handler(
	Query(params): Query<StreamParams>,
) -> Result<Json<ccc_types::streams::StreamResponse>, StreamProxyError> {
	handle_stream_request(
		params,
		|now| now - Duration::days(60),
		|now| now,
		QueryClass::Archived,
	)
	.await
}

#[derive(thiserror::Error, Debug)]
pub enum StreamProxyError {
	#[error("error from generic proxy: {0}")]
	GenericProxy(ccc_proxy::ProxyError),
}

impl IntoResponse for StreamProxyError {
	fn into_response(self) -> Response {
		let text = self.to_string();
		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(text.into())
			.unwrap()
	}
}
