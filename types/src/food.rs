use std::collections::HashMap;

use serde::{Deserialize, Serialize};
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
	cor_icons: HashMap<String, BonAppCorIcon>,
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
	cafes: HashMap<String, BonAppCafeResponse>,
}

impl BonAppCafesResponse {
	pub fn cafes_mut(&mut self) -> &mut HashMap<String, BonAppCafeResponse> {
		&mut self.cafes
	}
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
	cor_icons: HashMap<String, BonAppCorIcon>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BonAppCorIcon {
	#[ts(type = "any")]
	sort: Option<String>,
	label: String,
	description: String,
	image: String,
	is_filter: YesNo,
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
pub enum YesNo {
	N,
	Y,
}

#[derive(Serialize, Deserialize)]
pub struct BonAppMenuMultipleCafesResponse {
	days: Vec<BonAppMenuDayMultipleCafes>,
	items: HashMap<String, BonAppMenuItem>,
	cor_icons: HashMap<String, CorIconValue>,
	version: i64,
}

#[derive(Serialize, Deserialize)]
pub struct BonAppMenuSingleCafeResponse {
	days: Vec<BonAppMenuDaySingleCafe>,
	items: HashMap<String, BonAppMenuItem>,
	cor_icons: HashMap<String, CorIconValue>,
	version: i64,
}

impl BonAppMenuMultipleCafesResponse {
	pub fn as_single_day_response(self, cafe: &str) -> BonAppMenuSingleCafeResponse {
		let items = self.items;
		let cor_icons = self.cor_icons;
		let version = self.version;

		let days = self
			.days
			.into_iter()
			.filter_map(|day| day.into_single_day_response(cafe))
			.collect();

		BonAppMenuSingleCafeResponse {
			days,
			items,
			cor_icons,
			version,
		}
	}
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Connector(String);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NumericString(String);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HtmlString(String);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CurrencyString(String);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MonotonyContainer(Monotony);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Monotony {
	id: String,
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	short_name: Option<String>,
	image: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NutritionContainer(Nutrition);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Nutrition {
	kcal: NumericString,
	well_being: String,
	well_being_image: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NutritionDetailContainer(ItemNutritionDetails);

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ItemNutritionDetails {
	#[serde(skip_serializing_if = "Option::is_none")]
	calories: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	serving_size: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	fat_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	saturated_fat_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	trans_fat_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	cholesterol_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	sodium_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	carbohydrate_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	fiber_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	sugar_content: Option<NutritionDetail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	protein_content: Option<NutritionDetail>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NutritionDetailType(NutritionDetail);

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NutritionDetail {
	label: String,
	value: f32,
	unit: String,
}

////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct BonAppMenuItem {
	connector: Connector,
	cor_icon: CorIconUnion,
	description: String,
	id: String,
	label: String,
	monotony: Monotony,
	nutrition: Nutrition,
	#[serde(skip_serializing_if = "Option::is_none")]
	nutrition_details: Option<ItemNutritionDetails>,
	nutrition_link: String,
	options: Vec<serde_json::Value>,
	price: CurrencyString,
	rating: NumericString,
	special: bool,
	station: HtmlString,
	sub_station: String,
	sub_station_id: NumericString,
	sub_station_order: NumericString,
	tier3: bool,
	zero_entree: NumericString,
}

////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct CorIconValue {
	sort: String,
	label: String,
	description: String,
	image: String,
	is_filter: String,
	allergen: i64,
	#[serde(rename = "type")]
	cor_icon_type: String,
	position: String,
	show_name_ds: String,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct BonAppMenuDayMultipleCafes {
	date: String,
	cafes: HashMap<String, CafeDayMenu>,
}

impl BonAppMenuDayMultipleCafes {
	fn into_single_day_response(mut self, cafe: &str) -> Option<BonAppMenuDaySingleCafe> {
		let date = self.date;
		self
			.cafes
			.remove(cafe)
			.map(|cafe| BonAppMenuDaySingleCafe { date, cafe })
	}
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct BonAppMenuDaySingleCafe {
	date: String,
	cafe: CafeDayMenu,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct CafeDayMenu {
	name: String,
	comma_operator: String,
	pipe_operator: String,
	menu_id: String,
	dayparts: Vec<Vec<Daypart>>,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct Daypart {
	starttime: String,
	endtime: String,
	id: String,
	label: String,
	abbreviation: String,
	message: String,
	stations: Vec<StationElement>,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct StationElement {
	order_id: String,
	id: String,
	label: String,
	price: String,
	note: String,
	soup: i64,
	image: String,
	items: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct OrderedCorIconValue {
	id: String,
	label: String,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct Recipes {
	entrees: Vec<Entree>,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct Entree {
	recipe_id: String,
	recipe_name: String,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct Size {
	size: String,
	portion: String,
	uom: String,
	price: String,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct ItemNutrition {
	id: String,
	label: String,
	description: String,
	zero_entree: String,
	raw_cooked: String,
	is_rotating: String,
	cor_icon: CorIconUnion,
	price: String, // should change other Price to be String and not enum
	sizes: Vec<Size>,
	nutrition_summary: Nutrition,
	nutrition_details: ItemNutritionDetails,
	ingredients: String,
	ingredient_details: String,
	nutrition_info: String,
	special: i64,
	rating: String,
	connector: String,
	options: OptionsUnion,
	vendors: Vec<Option<serde_json::Value>>, // should change this to somewhat match OptionsUnion
	monotony: Monotony,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct ItemNutritionResponse {
	items: HashMap<String, ItemNutrition>,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct OptionsClass {
	label: String,
	#[serde(rename = "type")]
	options_type: String,
	values: Vec<OptionValue>,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub struct OptionValue {
	label: String,
	description: String,
	nutrition: Monotony,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[deprecated = "still need to check"]
pub enum CorIconUnion {
	AnythingArray(Vec<Option<serde_json::Value>>),
	EnumMap(HashMap<String, String>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[deprecated = "still need to check"]
pub enum OrderedCorIconUnion {
	AnythingArray(Vec<Option<serde_json::Value>>),
	OrderedCorIconValueMap(HashMap<String, OrderedCorIconValue>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[deprecated = "still need to check"]
pub enum OptionsUnion {
	AnythingArray(Vec<Option<serde_json::Value>>),
	OptionsClass(OptionsClass),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[deprecated = "still need to check"]
pub enum Tier {
	Integer(i64),
	String(String),
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub enum CaloriesLabel {
	Calories,
	Cholesterol,
	#[serde(rename = "Dietary Fiber")]
	DietaryFiber,
	Protein,
	#[serde(rename = "Saturated Fat")]
	SaturatedFat,
	#[serde(rename = "Serving Size")]
	ServingSize,
	Sodium,
	Sugars,
	#[serde(rename = "Total Carbohydrate")]
	TotalCarbohydrate,
	#[serde(rename = "Total Fat")]
	TotalFat,
	#[serde(rename = "Trans Fat")]
	TransFat,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub enum Unit {
	#[serde(rename = "")]
	Empty,
	#[serde(rename = "g")]
	G,
	#[serde(rename = "mg")]
	Mg,
	#[serde(rename = "oz")]
	Oz,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub enum NutritionLink {
	#[serde(rename = "")]
	Empty,
	#[serde(rename = "nutrition information")]
	NutritionInformation,
}

#[derive(Serialize, Deserialize)]
#[deprecated = "still need to check"]
pub enum Price {
	#[serde(rename = "")]
	Empty,
	#[serde(rename = "&nbsp;")]
	Nbsp,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[deprecated = "still need to check"]
pub enum KCalEnum {
	String(String),
	Integer(i64),
}
