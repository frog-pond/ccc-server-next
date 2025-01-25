use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "Stream")]
pub struct StreamEntry {
	pub starttime: String,
	pub location: String,
	pub eid: String,
	pub performer: String,
	pub subtitle: String,
	pub poster: String,
	pub player: String,
	pub status: String,
	pub category: String,
	pub hptitle: String,
	pub category_textcolor: Option<String>,
	pub category_color: Option<String>,
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
