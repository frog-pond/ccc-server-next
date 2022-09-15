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
