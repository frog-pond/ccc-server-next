//! Runs a prepared request through retrieval, response adaptation, and conversion.
//!
//! Application setup assembles compatible stages into a [`pipeline::Handle`].
//! During a request, [`operation::Operation`] uses that handle to produce a
//! domain value and report how it was obtained. The application owns request
//! preparation, authorization, and presentation.

#![allow(clippy::todo, dead_code, unused_variables)] // Types review draft (rubric E3).

pub mod body;
pub mod future;
pub mod operation;
pub mod pipeline;
pub mod source;
pub mod transform;
pub mod uncached;
