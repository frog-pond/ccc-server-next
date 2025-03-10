use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StavMealtimeReport(pub Vec<StavMealtimeDay>);

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StavMealtimeDay {
	pub date: String,
	pub times: HashMap<String, u32>,
}
