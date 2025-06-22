use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct WpJsonResponse {
	pub posts: Option<Vec<WpJsonPost>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpJsonPost {
	pub id: Option<u64>,
	pub date: Option<String>,
	pub date_gmt: Option<String>,
	pub guid: Option<WpJsonGuid>,
	pub modified: Option<String>,
	pub modified_gmt: Option<String>,
	pub slug: Option<String>,
	pub status: Option<String>,
	#[serde(rename = "type")]
	pub type_: Option<String>,
	pub link: Option<String>,
	pub title: Option<WpJsonTitle>,
	pub content: Option<WpJsonContent>,
	pub excerpt: Option<WpJsonExcerpt>,
	pub author: Option<u64>,
	pub featured_media: Option<u64>,
	pub comment_status: Option<String>,
	pub ping_status: Option<String>,
	pub sticky: Option<bool>,
	pub template: Option<String>,
	pub format: Option<String>,
	pub meta: Option<Value>,
	pub categories: Option<Vec<u64>>,
	pub tags: Option<Vec<u64>>,
	pub class_list: Option<Vec<String>>,
	pub acf: Option<Value>,
	pub jetpack_featured_media_url: Option<String>,
	pub jetpack_sharing_enabled: Option<bool>,
	#[serde(rename = "_links")]
	pub links: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpJsonGuid {
	pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpJsonTitle {
	pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpJsonContent {
	pub rendered: Option<String>,
	pub protected: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpJsonExcerpt {
	pub rendered: Option<String>,
	pub protected: Option<bool>,
}
