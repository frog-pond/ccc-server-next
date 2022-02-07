use crate::hours_handler;
use axum::{routing::get, Router};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    hours: Vec<Hour>,
    #[serde(rename = "closedForChapel")]
    closed_for_chapel_time: Option<bool>,
    #[serde(rename = "isPhysicallyOpen")]
    is_physically_open: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum DayOfWeek {
    Mo,
    Tu,
    We,
    Th,
    Fr,
    Sa,
    Su,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Hour {
    days: Vec<DayOfWeek>,
    from: String,
    to: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BreakSchedule {
    fall: Vec<Schedule>,
    thanksgiving: Vec<Schedule>,
    winter: Vec<Schedule>,
    interim: Vec<Schedule>,
    spring: Vec<Schedule>,
    easter: Vec<Schedule>,
    summer: Vec<Schedule>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct HoursItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    category: String,
    schedule: Vec<Schedule>,
    #[serde(rename = "breakSchedule")]
    break_schedule: BreakSchedule,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct HoursResponse {
    data: Vec<HoursItem>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/hours", get(hours_handler))
}
