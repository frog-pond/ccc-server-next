use std::sync::OnceLock;

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http::StatusCode;
use reqwest::{header::HeaderValue, Request};
use serde::de::DeserializeOwned;
use tracing::instrument;

pub struct Proxy {
	client: reqwest::Client,
}

pub static GLOBAL_PROXY: OnceLock<Proxy> = OnceLock::new();

pub fn global_proxy() -> &'static Proxy {
	GLOBAL_PROXY.get_or_init(Proxy::default)
}

fn user_agent() -> HeaderValue {
	let user_agent = match std::env::var("CCC_USER_AGENT") {
		Ok(env_variable) => env_variable,
		Err(_) => format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
	};

	HeaderValue::from_str(&user_agent).expect("user-agent should be a valid HeaderValue")
}

impl Default for Proxy {
	fn default() -> Self {
		let client = reqwest::Client::builder()
			.user_agent(user_agent())
			.build()
			.unwrap();

		Self { client }
	}
}

impl Proxy {
	pub fn client(&self) -> &reqwest::Client {
		&self.client
	}

	#[instrument(skip_all, fields(http.method = request.method().to_string(), http.uri = request.url().to_string()))]
	pub async fn send_request(&self, request: Request) -> Result<Bytes, ProxyError> {
		tracing::trace!(?request, "sending proxied request");

		let response = self
			.client
			.execute(request)
			.await
			.map_err(ProxyError::ProxiedRequest)?;

		tracing::trace!(http.status_code = %response.status(), "received proxied response");

		let response = response
			.bytes()
			.await
			.map_err(ProxyError::ProxiedResponse)?;

		tracing::trace!(bytes = %response.len(), "collected proxied response bytes");

		Ok(response)
	}

	#[instrument(skip_all, fields(http.method = request.method().to_string(), http.uri = request.url().to_string()))]
	pub async fn send_request_parse_json<T>(&self, request: Request) -> Result<T, ProxyError>
	where
		T: DeserializeOwned,
	{
		let bytes: Bytes = self.send_request(request).await?;

		let string = core::str::from_utf8(&bytes).map(|str| str.to_string());
		let parsed = serde_json::from_slice(&bytes);

		match (parsed, string) {
			(Ok(object), _) => Ok(object),
			(Err(parse_err), Ok(str)) => Err(ProxyError::ParseProxiedResponse(parse_err, str)),
			(Err(parse_err), Err(_e)) => Err(ProxyError::ParseProxiedResponse(
				parse_err,
				"<invalid utf8>".to_string(),
			)),
		}
	}
}

#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
	#[error("error sending proxied request: {0}")]
	ProxiedRequest(reqwest::Error),

	#[error("error receiving proxied response: {0}")]
	ProxiedResponse(reqwest::Error),

	#[error("error parsing proxied response data as JSON: {0}\n{1}")]
	ParseProxiedResponse(serde_json::Error, String),
}

impl IntoResponse for ProxyError {
	fn into_response(self) -> Response {
		let text = self.to_string();

		let body = text.into();

		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(body)
			.unwrap()
	}
}
