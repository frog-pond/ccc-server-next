use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use ts_rs::TS;

use std::collections::HashMap;

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

//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//

#[derive(Serialize, Deserialize)]
pub struct BonAppMenuResponse {
	days: Vec<Day>,
	items: HashMap<String, Item>,
	superplates: Vec<Option<serde_json::Value>>,
	goitems: HashMap<String, Goitem>,
	cor_icons: HashMap<String, CorIconValue>,
	version: i64,
}

#[derive(Serialize, Deserialize)]
pub struct CorIconValue {
	sort: String,
	label: LabelValue,
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
pub struct Day {
	date: String,
	cafes: HashMap<String, CafeDayMenu>,
}

#[derive(Serialize, Deserialize)]
pub struct CafeDayMenu {
	name: String,
	comma_operator: String,
	pipe_operator: String,
	menu_id: String,
	dayparts: Vec<Vec<Daypart>>,
}

#[derive(Serialize, Deserialize)]
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
pub struct Goitem {
	id: String,
	label: String,
	description: String,
	short_name: String,
	recipes: Recipes,
	barcode: String,
	raw_cooked: String,
	is_rotating: String,
	zero_entree: String,
	cor_icon: CorIconUnion,
	ordered_cor_icon: OrderedCorIconUnion,
	contains_statement: String,
	nextepid: String,
	price: String,
	sizes: Vec<Size>,
	nutrition: Nutrition,
	special: i64,
	tier3: i64,
	tier: i64,
	rating: String,
	connector: String,
	options: Vec<Option<serde_json::Value>>,
	station_id: String,
	station: String,
	nutrition_details: GoitemNutritionDetails,
	ingredients: String,
	nutrition_link: String,
	sub_station_id: String,
	sub_station: String,
	sub_station_order: String,
	monotony: Vec<Option<serde_json::Value>>,
	is_orderable: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Nutrition {
	kcal: String,
	well_being: Option<String>,
	well_being_image: String,
}

#[derive(Serialize, Deserialize)]
pub struct GoitemNutritionDetails {
	calories: Calories,
	#[serde(rename = "servingSize")]
	serving_size: Calories,
	#[serde(rename = "fatContent")]
	fat_content: Calories,
	#[serde(rename = "saturatedFatContent")]
	saturated_fat_content: Calories,
	#[serde(rename = "transFatContent")]
	trans_fat_content: Calories,
	#[serde(rename = "cholesterolContent")]
	cholesterol_content: Calories,
	#[serde(rename = "sodiumContent")]
	sodium_content: Calories,
	#[serde(rename = "carbohydrateContent")]
	carbohydrate_content: Calories,
	#[serde(rename = "fiberContent")]
	fiber_content: Calories,
	#[serde(rename = "sugarContent")]
	sugar_content: Calories,
	#[serde(rename = "proteinContent")]
	protein_content: Calories,
}

#[derive(Serialize, Deserialize)]
pub struct Calories {
	label: CaloriesLabel,
	value: String,
	unit: Unit,
}

#[derive(Serialize, Deserialize)]
pub struct OrderedCorIconValue {
	id: String,
	label: LabelValue,
}

#[derive(Serialize, Deserialize)]
pub struct Recipes {
	entrees: Vec<Entree>,
}

#[derive(Serialize, Deserialize)]
pub struct Entree {
	recipe_id: String,
	recipe_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Size {
	size: String,
	portion: String,
	uom: String,
	price: String,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
	id: String,
	label: String,
	recipes: Option<Recipes>,
	description: String,
	short_name: String,
	raw_cooked: String,
	is_rotating: String,
	zero_entree: String,
	cor_icon: CorIconUnion,
	ordered_cor_icon: OrderedCorIconUnion,
	nextepid: Option<String>,
	price: Price,
	sizes: Vec<Size>,
	nutrition: Nutrition,
	special: i64,
	tier3: i64,
	tier: Tier,
	rating: String,
	connector: Connector,
	options: OptionsUnion,
	station_id: String,
	station: StationEnum,
	nutrition_details: ItemNutritionDetails,
	ingredients: String,
	nutrition_link: NutritionLink,
	sub_station_id: String,
	sub_station: String,
	sub_station_order: String,
	monotony: Monotony,
}

#[derive(Serialize, Deserialize)]
pub struct Monotony {}

#[derive(Serialize, Deserialize)]
pub struct ItemNutritionDetails {
	calories: Option<Calories>,
	#[serde(rename = "servingSize")]
	serving_size: Option<Calories>,
	#[serde(rename = "fatContent")]
	fat_content: Option<Calories>,
	#[serde(rename = "saturatedFatContent")]
	saturated_fat_content: Option<Calories>,
	#[serde(rename = "transFatContent")]
	trans_fat_content: Option<Calories>,
	#[serde(rename = "cholesterolContent")]
	cholesterol_content: Option<Calories>,
	#[serde(rename = "sodiumContent")]
	sodium_content: Option<Calories>,
	#[serde(rename = "carbohydrateContent")]
	carbohydrate_content: Option<Calories>,
	#[serde(rename = "fiberContent")]
	fiber_content: Option<Calories>,
	#[serde(rename = "sugarContent")]
	sugar_content: Option<Calories>,
	#[serde(rename = "proteinContent")]
	protein_content: Option<Calories>,
}

#[derive(Serialize, Deserialize)]
pub struct OptionsClass {
	label: String,
	#[serde(rename = "type")]
	options_type: String,
	values: Vec<OptionValue>,
}

#[derive(Serialize, Deserialize)]
pub struct OptionValue {
	label: String,
	description: String,
	nutrition: Monotony,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum CorIconUnion {
	AnythingArray(Vec<Option<serde_json::Value>>),
	EnumMap(HashMap<String, LabelValue>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderedCorIconUnion {
	AnythingArray(Vec<Option<serde_json::Value>>),
	OrderedCorIconValueMap(HashMap<String, OrderedCorIconValue>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionsUnion {
	AnythingArray(Vec<Option<serde_json::Value>>),
	OptionsClass(OptionsClass),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tier {
	Integer(i64),
	String(String),
}

#[derive(Serialize, Deserialize)]
pub enum LabelValue {
	#[serde(rename = "Farm to Fork")]
	FarmToFork,
	Halal,
	Humane,
	#[serde(rename = "Made without Gluten-Containing Ingredients")]
	MadeWithoutGlutenContainingIngredients,
	Organic,
	#[serde(rename = "Supplier Diversity")]
	SupplierDiversity,
	Vegan,
	Vegetarian,
}

#[derive(Serialize, Deserialize)]
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
pub enum Connector {
	#[serde(rename = "and")]
	And,
	#[serde(rename = "")]
	Empty,
}

#[derive(Serialize, Deserialize)]
pub enum NutritionLink {
	#[serde(rename = "")]
	Empty,
	#[serde(rename = "nutrition information")]
	NutritionInformation,
}

#[derive(Serialize, Deserialize)]
pub enum Price {
	#[serde(rename = "")]
	Empty,
	#[serde(rename = "&nbsp;")]
	Nbsp,
}

#[derive(Serialize, Deserialize)]
pub enum StationEnum {
	#[serde(rename = "<strong>@bowls</strong>")]
	StrongBowlsStrong,
	#[serde(rename = "<strong>@cereal</strong>")]
	StrongCerealStrong,
	#[serde(rename = "<strong>@deli</strong>")]
	StrongDeliStrong,
	#[serde(rename = "<strong>@grains</strong>")]
	StrongGrainsStrong,
	#[serde(rename = "<strong>@grill</strong>")]
	StrongGrillStrong,
	#[serde(rename = "<strong>@home</strong>")]
	StrongHomeStrong,
	#[serde(rename = "<strong>@market salads</strong>")]
	StrongMarketSaladsStrong,
	#[serde(rename = "<strong>@pasta</strong>")]
	StrongPastaStrong,
	#[serde(rename = "<strong>@pizza</strong>")]
	StrongPizzaStrong,
	#[serde(rename = "<strong>@soup</strong>")]
	StrongSoupStrong,
	#[serde(rename = "<strong>@sweet treats</strong>")]
	StrongSweetTreatsStrong,
	#[serde(rename = "<strong>@Toaster</strong>")]
	StrongToasterStrong,
	#[serde(rename = "<strong>@tortilla</strong>")]
	StrongTortillaStrong,
}
