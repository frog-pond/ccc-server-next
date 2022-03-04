use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Schedule {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    hours: Vec<Hour>,
    closed_for_chapel_time: Option<bool>,
    is_physically_open: Option<bool>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DayOfWeek {
    Mo,
    Tu,
    We,
    Th,
    Fr,
    Sa,
    Su,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Hour {
    days: Vec<DayOfWeek>,
    from: String,
    to: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BreakSchedule {
    fall: Vec<Schedule>,
    thanksgiving: Vec<Schedule>,
    winter: Vec<Schedule>,
    interim: Vec<Schedule>,
    spring: Vec<Schedule>,
    easter: Vec<Schedule>,
    summer: Vec<Schedule>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HoursItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    category: String,
    schedule: Vec<BreakSchedule>,
    break_schedule: BreakSchedule,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HoursResponse {
    data: Vec<HoursItem>,
}
