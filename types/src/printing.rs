use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ColorPrintersResponseItem")]
pub struct ResponseItem {
    color_printers: Vec<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ColorPrintersResponse")]
pub struct Response {
    data: ResponseItem,
}
