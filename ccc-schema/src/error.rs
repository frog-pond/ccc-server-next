//! JSON errors produced when an HTTP router cannot match a request.
//!
//! The native and Worker entry points serialize this body and choose the HTTP
//! status. The shape comes from the reference's
//! [Worker router](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/router.ts#L372).

use serde::Serialize;

/// JSON body of a routing failure, with exactly one `error` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RouteErrorBody {
	/// The message that describes why routing failed.
	pub error: RouteError,
}

/// The fixed messages for requests that the router cannot match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RouteError {
	/// No route matches the requested path. Serialized as `"not found"`.
	#[serde(rename = "not found")]
	NotFound,
	/// The request used an unsupported method. Serialized as `"method not allowed"`.
	#[serde(rename = "method not allowed")]
	MethodNotAllowed,
}
