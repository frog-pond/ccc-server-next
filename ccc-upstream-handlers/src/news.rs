use axum::{
	extract::Query,
	response::{IntoResponse, Response},
	Json,
};
use http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

use ccc_upstream_proxy::ProxyError;

#[derive(thiserror::Error, Debug)]
pub enum NewsProxyError {
	#[error("error from proxy: {0}")]
	Proxy(ProxyError),

	#[error("missing required parameter: {0}")]
	MissingParameter(String),
}

impl IntoResponse for NewsProxyError {
	fn into_response(self) -> Response {
		let (status, text) = match &self {
			NewsProxyError::MissingParameter(_) => (StatusCode::BAD_REQUEST, self.to_string()),
			NewsProxyError::Proxy(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
		};

		let body = text.into();

		Response::builder().status(status).body(body).unwrap()
	}
}

#[derive(Debug, Deserialize)]
pub struct WpJsonQuery {
	url: Option<String>,
}

#[instrument(skip_all)]
pub async fn wp_json_handler(
	Query(query): Query<WpJsonQuery>,
) -> Result<Json<Vec<ccc_types::news::WpJsonPost>>, NewsProxyError> {
	let url = query
		.url
		.ok_or_else(|| NewsProxyError::MissingParameter("url".to_string()))?;

	let request = ccc_upstream_proxy::global_proxy()
		.client()
		.get(url)
		.build()
		.map_err(ProxyError::ProxiedRequest)
		.map_err(NewsProxyError::Proxy)?;

	ccc_upstream_proxy::global_proxy()
		.send_request_parse_json::<Vec<ccc_types::news::WpJsonPost>>(request)
		.await
		.map(Json)
		.map_err(NewsProxyError::Proxy)
}
