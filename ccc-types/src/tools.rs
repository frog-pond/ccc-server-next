use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HelpButton {
	title: String,
	action: String,
	params: HelpParams,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HelpParams {
	#[serde(skip_serializing_if = "Option::is_none")]
	url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	number: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	to: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	subject: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	body: Option<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "HelpResponseItem")]
pub struct ResponseItem {
	key: String,
	enabled: bool,
	hidden: bool,
	title: String,
	body: String,
	buttons: Vec<HelpButton>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "HelpResponse")]
pub struct Response {
	data: Vec<ResponseItem>,
}
