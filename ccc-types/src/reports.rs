use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct StavMealtimeReport(pub Vec<StavMealtimeDay>);

#[derive(Debug, Serialize, Deserialize)]
pub struct StavMealtimeDay {
	pub date: String,
	pub times: HashMap<String, u32>,
}
