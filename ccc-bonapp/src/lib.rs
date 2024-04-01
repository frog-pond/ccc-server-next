//! # BonApp
//!
//! BonApp runs an API at base endpoint `https://legacy.cafebonappetit.com/api/2/`.
//!
//! The API has three main endpoints:
//! - `cafes` - returning cafe-level information
//! - `menus` - returning expanded information about the menus
//! - `items` - returning nutrition information about items
//!
//! ## Cafe queries
//!
//! Cafe queries are made by providing a numeric-small BonApp Cafe ID to the `cafes` endpoint.
//! You can also supply multiple cafe ids to the cafes endpoint to get information about multiple cafes.
//!
//! The response is of the form:
//!
//! ```json
//! $ curl -s 'https://legacy.cafebonappetit.com/api/2/cafes?cafe=1,2' | jq .
//! {
//!     "cafes": {
//!         "1": { ... },
//!         "2": { ... },
//!     }
//! }
//! ```
