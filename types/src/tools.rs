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
    url: Option<String>,
    number: Option<String>,
    to: Option<String>,
    subject: Option<String>,
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
