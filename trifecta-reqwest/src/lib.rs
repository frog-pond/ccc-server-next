//! Native HTTP retrieval and whole-body adaptation for Trifecta.
//!
//! The native entry point supplies a configured [`reqwest::Client`]. Its
//! client configuration controls redirects, timeouts, and proxy behavior.

#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::todo, dead_code, unused_variables)] // Types review draft (rubric E3).

use std::{error::Error as StdError, fmt};

use reqwest::{header::HeaderMap, Client, Request, Response, StatusCode};
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	source::Source,
};

/// Retrieves an HTTP response using an injected client and request constructor.
///
/// The constructor receives the application's prepared request and returns a
/// fresh HTTP request for each invocation, including any headers or body it
/// needs. The application's setup and preparation own any fallible validation.
pub struct HttpSource<F> {
	client: Client,
	request: F,
}

impl<F> HttpSource<F> {
	/// Uses `client` to execute the HTTP requests produced by `request`.
	pub fn new<P>(client: Client, request: F) -> Self
	where
		F: Fn(&P) -> Request,
	{
		todo!()
	}
}

impl<P, F> Source<P> for HttpSource<F>
where
	F: Fn(&P) -> Request + Send + Sync + 'static,
{
	type Response = Response;
	type Error = Error;

	fn retrieve<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Result<Response, Error>> {
		todo!()
	}
}

/// The upstream response status and headers, moved out of the native response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpMetadata {
	/// The status returned by the upstream, including non-success statuses.
	pub status: StatusCode,
	/// The upstream response headers, including repeated header values.
	pub headers: HeaderMap,
}

/// Buffers a reqwest response and carries its status and headers to the transform.
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpResponseAdapter;

impl AdaptResponse<Response> for HttpResponseAdapter {
	type Metadata = HttpMetadata;
	type Error = Error;

	fn adapt_response(
		&self,
		response: Response,
	) -> BoxFuture<'_, Result<BodyWithMetadata<HttpMetadata>, Error>> {
		todo!()
	}
}

/// A transport failure while acquiring an HTTP response or buffering its body.
///
/// The reqwest cause retains its error classification. Its request URL is
/// removed before the error leaves this integration.
#[derive(Debug)]
pub enum Error {
	/// The request failed before reqwest returned an HTTP response.
	Execute(reqwest::Error),
	/// The response arrived, but its body could not be read to completion.
	ReadBody(reqwest::Error),
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		todo!()
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		todo!()
	}
}
