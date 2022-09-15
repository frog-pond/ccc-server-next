const BONAPP_CAFE_KEY_STAV: &str = "stav-hall";
const BONAPP_CAFE_KEY_CAGE: &str = "the-cage";
const BONAPP_CAFE_KEY_KINGS_ROOM: &str = "kings-room";
const BONAPP_CAFE_KEY_BURTON: &str = "burton";
const BONAPP_CAFE_KEY_LDC: &str = "ldc";
const BONAPP_CAFE_KEY_SAYLES: &str = "sayles";
const BONAPP_CAFE_KEY_WEITZ: &str = "weitz";

#[derive(Debug)]
enum NamedBonAppCafe {
	Stav,
	Cage,
	KingsRoom,
	Burton,
	Ldc,
	Sayles,
	Weitz,
}

impl NamedBonAppCafe {
	fn from_name(name: &str) -> Option<Self> {
		use NamedBonAppCafe::*;
		match name {
			BONAPP_CAFE_KEY_STAV => Some(Stav),
			BONAPP_CAFE_KEY_CAGE => Some(Cage),
			BONAPP_CAFE_KEY_KINGS_ROOM => Some(KingsRoom),
			BONAPP_CAFE_KEY_BURTON => Some(Burton),
			BONAPP_CAFE_KEY_LDC => Some(Ldc),
			BONAPP_CAFE_KEY_SAYLES => Some(Sayles),
			BONAPP_CAFE_KEY_WEITZ => Some(Weitz),
			_ => None,
		}
	}

	const fn get_bonapp_cafe_id(&self) -> u32 {
		use NamedBonAppCafe::*;
		match self {
			Stav => 261,
			Cage => 262,
			KingsRoom => 263,
			Burton => 35,
			Ldc => 36,
			Sayles => 24,
			Weitz => 458,
		}
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum ProxyRequestQueryParameters {
	CafeQuery { cafe: String },
	MenuQuery { cafe: String },
	ItemNutritionQuery { item: String },
}

#[test]
fn query_parameters_serialize() {
	let query = ProxyRequestQueryParameters::CafeQuery {
		cafe: "foo".to_string(),
	};
	assert_eq!(
		serde_urlencoded::to_string(query).ok(),
		Some("cafe=foo".to_string())
	);
}
