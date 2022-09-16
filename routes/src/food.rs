use axum::{routing::get, Router};
use handlers::{
	bonapp::{cafe_handler, cafe_menu_handler, named_cafe_handler, named_cafe_menu_handler},
	github::pause_menu_handler,
};

#[must_use]
pub fn router() -> Router {
	Router::new()
		.route("/menu/:cafe_id", get(cafe_menu_handler))
		.route("/cafe/:cafe_id", get(cafe_handler))
		.route("/named/cafe/:name", get(named_cafe_handler))
		.route("/named/menu/:name", get(named_cafe_menu_handler))
		.route("/named/menu/the-pause", get(pause_menu_handler))
}
