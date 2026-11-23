use axum::{
	body::{to_bytes, Body},
	http::{header, Method, Request, StatusCode},
	response::Response,
};
use tower::ServiceExt;

use super::{build_http_client, router};

async fn request(method: Method, path: &str) -> Response {
	router(
		|| std::time::UNIX_EPOCH,
		build_http_client().expect("client"),
	)
	.oneshot(
		Request::builder()
			.method(method)
			.uri(path)
			.body(Body::empty())
			.expect("request"),
	)
	.await
	.expect("router")
}

#[tokio::test]
async fn post_ping_returns_exact_json_without_allow() {
	let response = request(Method::POST, "/ping").await;
	assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
	assert!(!response.headers().contains_key(header::ALLOW));
	assert_eq!(
		response
			.headers()
			.get(header::CONTENT_TYPE)
			.expect("content type"),
		"application/json"
	);
	let body = to_bytes(response.into_body(), 128).await.expect("body");
	assert_eq!(body.as_ref(), br#"{"error":"method not allowed"}"#);
}

#[tokio::test]
async fn head_ping_returns_405_without_allow() {
	let response = request(Method::HEAD, "/ping").await;
	assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
	assert!(!response.headers().contains_key(header::ALLOW));
	assert_eq!(
		response
			.headers()
			.get(header::CONTENT_TYPE)
			.expect("content type"),
		"application/json"
	);
	assert!(to_bytes(response.into_body(), 128)
		.await
		.expect("body")
		.is_empty());
}

#[tokio::test]
async fn unmatched_get_returns_exact_json() {
	for path in ["/missing", "/edu.stolaf/ping"] {
		let response = request(Method::GET, path).await;
		assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
		assert_eq!(
			response
				.headers()
				.get(header::CONTENT_TYPE)
				.expect("content type"),
			"application/json"
		);
		let body = to_bytes(response.into_body(), 128).await.expect("body");
		assert_eq!(body.as_ref(), br#"{"error":"not found"}"#);
	}
}

#[tokio::test]
async fn non_get_to_an_unmatched_path_returns_405() {
	let response = request(Method::POST, "/missing").await;
	assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
	assert!(!response.headers().contains_key(header::ALLOW));
	assert_eq!(
		response
			.headers()
			.get(header::CONTENT_TYPE)
			.expect("content type"),
		"application/json"
	);
	let body = to_bytes(response.into_body(), 128).await.expect("body");
	assert_eq!(body.as_ref(), br#"{"error":"method not allowed"}"#);
}

#[tokio::test]
async fn ping_returns_plain_text_after_path_normalization() {
	for path in ["/ping", "/x/../ping", "/%2e/ping"] {
		let response = request(Method::GET, path).await;
		assert_eq!(response.status(), StatusCode::OK, "{path}");
		assert_eq!(
			response
				.headers()
				.get(header::CONTENT_TYPE)
				.expect("content type"),
			"text/plain; charset=utf-8"
		);
		let body = to_bytes(response.into_body(), 128).await.expect("body");
		assert_eq!(body.as_ref(), b"pong");
	}
}
