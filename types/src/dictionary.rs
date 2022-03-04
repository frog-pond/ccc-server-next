use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "DictionaryResponseItem")]
pub struct ResponseItem {
    word: String,
    definition: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "DictionaryResponse")]
pub struct Response {
    data: Vec<ResponseItem>,
}
