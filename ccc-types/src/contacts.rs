use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ContactResponseItem")]
pub struct ResponseItem {
	title: String,
	phone_number: Option<String>,
	button_text: Option<String>,
	category: String,
	#[ts(optional)]
	image: Option<String>,
	synopsis: String,
	text: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ContactResponse")]
pub struct Response {
	data: Vec<ResponseItem>,
}
