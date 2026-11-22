//! Trifecta drives runtime-neutral and transport-neutral asynchronous
//! acquisition of domain types from external sources.
//!
//! A Trifecta application is built in terms of _operations_ that turn prepared
//! _requests_ into domain values through configured _pipelines_.
//!
//! # Execution model
//!
//! An uncached invocation has three application-defined stages:
//!
//! ```text
//! prepared P
//!    | Source::retrieve
//! native response
//!    | AdaptResponse::adapt_response
//! BodyWithMetadata<M>
//!    | Transform::transform
//! domain T
//! ```
//!
//! These boundaries are intentionally separate.
//!
//! A [`Source`](source::Source) returns its native response so a future cache
//! integration can inspect or store that representation.
//!
//! [`AdaptResponse`](body::AdaptResponse) removes transport-specific details
//! and produces a runtime-neutral body type.
//!
//! [`Transform`](transform::Transform) is then the one canonical conversion to
//! the domain result, whether the raw value eventually comes from a source or
//! a cache.
//!
//! At application setup, [`uncached::Pipeline`] checks that those three stages
//! have compatible types and erases their concrete implementation behind the
//! common [`pipeline::Handle`]. An [`operation::Operation`] owns either one
//! handle or a [`pipeline::Selector`] of preassembled handles. At invocation
//! time, [`operation::Operation::perform`] consumes the prepared request and
//! produces an [`operation::Outcome`].
//!
//! # Getting started
//!
//! Implement the three stages, assemble them once, and keep the operation for
//! repeated calls:
//!
//! ```
//! # use std::convert::Infallible;
//! # use bytes::Bytes;
//! # use futures_util::stream;
//! use trifecta::{
//!     body::{AdaptResponse, Body, BodyReadError, BodyWithMetadata},
//!     future::BoxFuture,
//!     operation::Operation,
//!     pipeline::Id,
//!     source::Source,
//!     transform::Transform,
//!     uncached,
//! };
//!
//! # struct MySource;
//! # impl Source<String> for MySource {
//! #     type Response = Bytes;
//! #     type Error = Infallible;
//! #     fn retrieve<'a>(
//! #         &'a self,
//! #         request: &'a String,
//! #     ) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
//! #         Box::pin(async move { Ok(Bytes::copy_from_slice(request.as_bytes())) })
//! #     }
//! # }
//! # struct MyResponseAdapter;
//! # impl AdaptResponse<Bytes> for MyResponseAdapter {
//! #     type Metadata = ();
//! #     type Error = Infallible;
//! #     fn adapt_response(
//! #         &self,
//! #         response: Bytes,
//! #     ) -> BoxFuture<'_, Result<BodyWithMetadata<Self::Metadata>, Self::Error>> {
//! #         Box::pin(async move {
//! #             let chunks = stream::once(async move { Ok::<_, BodyReadError>(response) });
//! #             Ok(BodyWithMetadata::new(Body::new(Box::pin(chunks)), ()))
//! #         })
//! #     }
//! # }
//! # struct MyTransform;
//! # impl Transform<String, (), usize> for MyTransform {
//! #     type Error = Infallible;
//! #     fn transform<'a>(
//! #         &'a self,
//! #         request: &'a String,
//! #         _body: BodyWithMetadata<()>,
//! #     ) -> BoxFuture<'a, Result<usize, Self::Error>> {
//! #         Box::pin(async move { Ok(request.len()) })
//! #     }
//! # }
//! # async fn example() -> Result<(), trifecta::operation::Error> {
//! let handle = uncached::Pipeline::new(MySource, MyResponseAdapter, MyTransform)
//!     .into_handle(Id::new("document-length"));
//! let operation: Operation<String, usize> = Operation::new(handle);
//!
//! let outcome = operation.perform("hello".to_owned()).await?;
//! assert_eq!(outcome.value(), &5);
//! # Ok(())
//! # }
//! ```
//!
//! # Ownership boundary
//!
//! Trifecta does not define a service trait or resource container. Applications
//! should keep shared clients and stores in their own service, prepare `P`
//! there, and keep `Operation<P, T>` as an implementation detail. This ensures
//! that authorization and caller-local work happen once per caller, outside any
//! future shared flight.
//!
//! # Module guide
//!
//! - [`source`] and [`body`] define acquisition and response adaptation.
//! - [`transform`] defines conversion to the operation's domain value.
//! - [`pipeline`] defines the common identity, erased handle, and selection
//!   machinery shared by all pipeline shapes.
//! - [`uncached`] assembles the pipeline shape with no cache. Pre-parse and
//!   post-parse caching belong in their own peer modules.
//! - [`operation`] executes a pipeline and reports its outcome.
//! - [`future`] contains the object-safe, runtime-neutral future type used by
//!   the extension traits.

#![warn(missing_docs)]

pub mod body;
pub mod future;
pub mod operation;
pub mod pipeline;
pub mod source;
pub mod transform;
pub mod uncached;
