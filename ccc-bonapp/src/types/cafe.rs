use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, TS)]
struct BonAppSingleCafeResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
struct BonAppCafesResponse {
	cafes: HashMap<String, BonAppSingleCafeResponse>,
}

#[test]
fn example_parse() {
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902469--cafe-261--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjYxCg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902470--cafe-262--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjYyCg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902470--cafe-263--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjYzCg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902470--cafe-35--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MzUK.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902471--cafe-24--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjQK.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902471--cafe-36--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MzYK.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902472--cafe-458--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9NDU4Cg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902689--cafe-261--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjYxCg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902689--cafe-262--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjYyCg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902690--cafe-263--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjYzCg==.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902690--cafe-35--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MzUK.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902690--cafe-36--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MzYK.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902691--cafe-24--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9MjQK.json")).is_ok());
	assert!(serde_json::from_str::<BonAppCafesResponse>(include_str!("../bonapp-examples/1711902691--cafe-458--LW4gaHR0cHM6Ly9sZWdhY3kuY2FmZWJvbmFwcGV0aXQuY29tL2FwaS8yL2NhZmVzP2NhZmU9NDU4Cg==.json")).is_ok());
}
