use axum::{routing::get, Router};
use handlers::{bonapp::named_cafe_handler, github::pause_menu_handler};

#[must_use]
pub fn router() -> Router {
	Router::new()
		.route("/named/cafe/:name", get(named_cafe_handler))
		.route("/named/menu/the-pause", get(pause_menu_handler))
}
