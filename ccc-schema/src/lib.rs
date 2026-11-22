//! Response types for the CCC API, shared by the native server and the Worker.
//!
//! Upstream adapters construct these values, and the server serializes them for
//! the apps. The crate defines the app-facing schema and has no dependencies on
//! upstream readers, runtimes, or web frameworks.
//!
//! See `docs/types.md` in the repository for the layout and review rules.
