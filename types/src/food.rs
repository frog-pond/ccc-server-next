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
	// #[ts(type = "Array<any>")]
	cor_icons: std::collections::HashMap<String, BonAppCorIcon>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PauseMenuResponse {
	data: PauseMenuItemResponse,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct BonAppCafesResponse {
	// #[ts(type = "Map<String, BonAppCafeResponse>")]
	cafes: std::collections::HashMap<String, BonAppCafeResponse>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct BonAppCafeResponse {
	name: String,
	address: String,
	city: String,
	state: String,
	zip: String,
	latitude: String,
	longitude: String,
	description: String, // html
	message: String,     // html
	eod: String,         // 02:00
	timezone: String,    // America/Chicago
	menu_type: String,   // dynamic
	menu_html: String,
	location_detail: String,
	weekly_schedule: String, // html
	days: Vec<BonAppDay>,
	cor_icons: std::collections::HashMap<String, BonAppCorIcon>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BonAppCorIcon {
	#[ts(type = "any")]
	sort: Option<serde_json::Value>,
	label: String,
	description: String,
	image: String,
	is_filter: IsFilter,
	allergen: i64,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BonAppDay {
	date: String,
	dayparts: Vec<BonAppDaypart>,
	status: String,
	message: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BonAppDaypart {
	id: String,
	starttime: String,
	endtime: String,
	message: Option<String>,
	label: String,
	hide: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub enum IsFilter {
	N,
	Y,
}
