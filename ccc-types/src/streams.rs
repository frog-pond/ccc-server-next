use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "Stream")]
pub struct StreamEntry {
	#[ts(type = "string")]
	pub starttime: DateTime<Utc>,
	pub location: String,
	#[ts(type = "any")]
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

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamResponse {
	pub results: Vec<StreamEntry>,
}
