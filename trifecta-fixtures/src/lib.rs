//! File-backed Trifecta retrieval for native offline tests.
//!
//! Tests inject a fixture directory and a function that selects a file for
//! the prepared request. A test can then reuse its normal response adapter
//! and transform wherever their input types match.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
	clippy::todo,
	dead_code,
	unused_variables,
	reason = "The types review draft declares the API before its implementation (rubric E3)"
)]

use std::{
	error::Error as StdError,
	fmt, io,
	path::{Path, PathBuf},
};

use bytes::Bytes;
use trifecta::{future::BoxFuture, source::Source};

/// Reads an entire fixture file from a test-provided directory.
///
/// File reads are blocking and meant for native tests. The selector yields
/// a relative path within the fixture tree. Tests supply trusted fixture
/// directories and their contents, including any symbolic links.
pub struct FixtureSource<F> {
	directory: PathBuf,
	path: F,
}

impl<F> FixtureSource<F> {
	/// Uses `path` to select a file under `directory` for each request.
	pub fn new<P>(directory: PathBuf, path: F) -> Self
	where
		F: Fn(&P) -> &Path,
	{
		todo!()
	}
}

impl<P, F> Source<P> for FixtureSource<F>
where
	P: Sync,
	F: Fn(&P) -> &Path + Send + Sync + 'static,
{
	type Response = Bytes;
	type Error = Error;

	fn retrieve<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Result<Bytes, Error>> {
		todo!()
	}
}

/// Why a test fixture could not be retrieved for the request.
#[derive(Debug)]
pub enum Error {
	/// The selected path is empty, absolute, or contains a parent component.
	InvalidPath(PathBuf),
	/// Reading the selected fixture failed.
	Read {
		/// Path of the file the source tried to read.
		path: PathBuf,
		/// The original filesystem error.
		source: io::Error,
	},
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
