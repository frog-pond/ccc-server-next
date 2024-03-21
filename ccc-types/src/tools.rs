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
	#[ts(optional)]
	url: Option<String>,
	#[ts(optional)]
	number: Option<String>,
	#[ts(optional)]
	to: Option<String>,
	#[ts(optional)]
	subject: Option<String>,
	#[ts(optional)]
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
