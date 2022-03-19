use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::collections::HashMap;
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModesItem {
	name: String,
	category: String,
	description: String,
	synopsis: String,
	url: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModesResponse {
	data: Vec<ModesItem>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BusTimesResponseItem {
	colors: TransitColors,
	line: String,
	notice: Option<String>,
	schedules: Vec<TransitSchedule>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransitColors {
	bar: String,
	dot: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransitSchedule {
	days: Vec<String>,
	coordinates: HashMap<String, [f64; 2]>,
	stops: Vec<String>,
	#[ts(type = "Array<Array<string|boolean>>")]
	times: Vec<Vec<Value>>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BusTimesResponse {
	data: Vec<BusTimesResponseItem>,
}
