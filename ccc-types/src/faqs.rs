use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export, rename = "FAQResponse")]
pub struct Response {
	text: String,
}
