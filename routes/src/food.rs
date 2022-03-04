use axum::{routing::get, Router};
use handlers::github::pause_menu_handler;

pub fn router() -> Router {
    Router::new().route("/named/menu/the-pause", get(pause_menu_handler))
}
