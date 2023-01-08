use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{BonAppCorIcon, ItemResponse, StationMenu};

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PauseMenuItemResponse {
	station_menus: Vec<StationMenu>,
	food_items: Vec<ItemResponse>,
	// #[ts(type = "Array<any>")]
	cor_icons: HashMap<String, BonAppCorIcon>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PauseMenuResponse {
	data: PauseMenuItemResponse,
}
