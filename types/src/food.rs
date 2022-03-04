use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "FoodStationMenu")]
pub struct StationMenu {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "FoodItemResponse")]
pub struct ItemResponse {
    label: String,
    station: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    special: Option<bool>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PauseMenuItemResponse {
    station_menus: Vec<StationMenu>,
    food_items: Vec<ItemResponse>,
    #[ts(type = "Array<any>")]
    cor_icons: Vec<Value>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PauseMenuResponse {
    data: PauseMenuItemResponse,
}
