use axum::{
	response::{IntoResponse, Response},
	Json,
};
use http::StatusCode;
use tracing::instrument;

use ccc_proxy::ProxyError;

#[derive(thiserror::Error, Debug)]
pub enum ReportsProxyError {
	#[error("error from proxy: {0}")]
	Proxy(ProxyError),
}

impl IntoResponse for ReportsProxyError {
	fn into_response(self) -> Response {
		let text = self.to_string();
		let body = text.into();

		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(body)
			.unwrap()
	}
}

#[instrument(skip_all)]
pub async fn stav_mealtime_report_handler(
) -> Result<Json<ccc_types::reports::StavMealtimeReport>, ReportsProxyError> {
	let raw_data = crate::github::stav_mealtime_handler()
		.await
		.map_err(ReportsProxyError::Proxy)?;

	let data_value = raw_data.0;
	let data_str = data_value.to_string();

	let report: ccc_types::reports::StavMealtimeReport =
		serde_json::from_value(data_value).map_err(|e| {
			ReportsProxyError::Proxy(ccc_proxy::ProxyError::ParseProxiedResponse(e, data_str))
		})?;

	Ok(Json(report))
}
