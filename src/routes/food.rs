use crate::pause_menu_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationMenu {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ItemResponse {
    label: String,
    station: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    special: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CorIcons {}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseMenuItemResponse {
    station_menus: Vec<StationMenu>,
    food_items: Vec<ItemResponse>,
    cor_icons: CorIcons,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PauseMenuResponse {
    data: PauseMenuItemResponse,
}

pub(crate) fn router() -> Router {
    Router::new().route("/named/menu/the-pause", get(pause_menu_handler))
}
