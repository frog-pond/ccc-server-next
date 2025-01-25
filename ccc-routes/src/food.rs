use axum::{routing::get, Router};
use ccc_handlers::{
	bonapp::{
		cafe_handler, cafe_menu_handler, named_cafe_handler, named_cafe_menu_handler, nutrition_handler,
	},
	github::pause_menu_handler,
};

pub fn router() -> Router {
	Router::new()
		.route("/menu/{cafe_id}", get(cafe_menu_handler))
		.route("/cafe/{cafe_id}", get(cafe_handler))
		.route("/item/{item_id}", get(nutrition_handler))
		.route("/named/cafe/{name}", get(named_cafe_handler))
		.route("/named/menu/the-pause", get(pause_menu_handler))
		.route("/named/menu/{name}", get(named_cafe_menu_handler))
}
