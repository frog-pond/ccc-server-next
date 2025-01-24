use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamEntry {
	pub starttime: DateTime<Utc>,
	pub location: String,
	pub eid: serde_json::Value,
	pub performer: String,
	pub subtitle: String,
	pub poster: String,
	pub player: String,
	pub status: String,
	pub category: String,
	pub hptitle: String,
	pub category_textcolor: String,
	pub category_color: String,
	pub thumb: String,
	pub title: String,
	pub iframesrc: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamResponse {
	pub results: Vec<StreamEntry>,
}
