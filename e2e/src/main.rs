use std::collections::{BTreeMap, BTreeSet};

use reqwest::{Client, ClientBuilder, Method, Request, Url};

fn sources(local_server: Option<&str>, deployed_js_server: Option<&str>) -> Vec<String> {
	let mut vec = Vec::default();

	if let Some(local_server) = local_server {
		vec.push(local_server.to_string());
	}

	if let Some(deployed_js_server) = deployed_js_server {
		vec.push(deployed_js_server.to_string());
	}

	vec
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Mode {
	StOlaf,
	Carleton,
}

fn get_substitutions(mode: &Mode, route: &str, token: &str) -> Vec<&'static str> {
	use Mode::{Carleton, StOlaf};

	match (mode, route, token) {
		(_, "food/item/:itemId", ":itemId") => vec!["36221", "13207271", "22061885"],
		(_, "food/menu/:cafeId", ":cafeId") => vec!["261", "262", "263", "35", "36", "24", "458"],
		(_, "food/cafe/:cafeId", ":cafeId") => vec!["261", "262", "263", "35", "36", "24", "458"],

		(Carleton, "convos/upcoming/:id", ":id") => vec![],

		// A panic in this case prompts us to fill out the substitution grid.
		//
		// To do so, add a new match arm for the mode, route, and specific path segment you want to add to.
		_ => {
			panic!(
				"unknown substitution {} for route {} in mode {}",
				token,
				route,
				match mode {
					StOlaf => "stolaf",
					Carleton => "carleton",
				}
			);
		}
	}
}

fn substitute(mode: &Mode, route: String) -> Vec<String> {
	let parts: Vec<&str> = route.split('/').collect();

	if parts.iter().any(|part| part.starts_with(':')) {
		todo!()
	} else {
		vec![parts.join("/")]
	}
}

fn routes(mode: &Mode) -> impl Iterator<Item = String> {
	let vec: Vec<String> = match mode {
		Mode::StOlaf => include_str!("../STOLAF_ROUTES")
			.lines()
			.map(ToOwned::to_owned),
		Mode::Carleton => include_str!("../CARLETON_ROUTES")
			.lines()
			.map(ToOwned::to_owned),
	}
	.flat_map(|configured_route| substitute(mode, configured_route))
	.collect();

	vec.into_iter()
}

fn create_request(
	client: &Client,
	mode: &Mode,
	source: &str,
	route: &str,
) -> Result<Request, Box<dyn std::error::Error + Send + Sync>> {
	let joined = Url::parse(source)?.join(route)?;
	Ok(client.request(Method::GET, joined).build()?)
}

fn test_targets() -> Result<Vec<(url::Url, Vec<Mode>)>, url::ParseError> {
	use Mode::{Carleton, StOlaf};

	Ok(vec![
		(
			Url::parse("http://localhost:3000/api/")?,
			vec![Carleton, StOlaf],
		),
		(
			Url::parse("https://stolaf.api.frogpond.tech/v1/")?,
			vec![StOlaf],
		),
		(
			Url::parse("https://carleton.api.frogpond.tech/v1/")?,
			vec![Carleton],
		),
	])
}

struct TestPlan();

impl TestPlan {
	fn generate(targets: Vec<(Url, Vec<Mode>)>) -> Self {
		// "targets" in this case is still structured in terms of (base_url, supported_modes).
		// to turn this into a "plan", we need to normalize in terms of (mode_route, servers_to_test).

		// First, convert targets into {Mode: Set<Url>}:
		let targets: BTreeMap<Mode, BTreeSet<Url>> = targets
			.into_iter()
			.flat_map(|(base_url, supported_modes)| {
				supported_modes
					.into_iter()
					.map(move |supported_mode| (supported_mode, base_url.clone()))
			})
			.fold(
				BTreeMap::default(),
				|mut map, (supported_mode, base_url)| {
					map
						.entry(supported_mode)
						.or_insert(BTreeSet::default())
						.insert(base_url);
					map
				},
			);

		// "Mode" comes with a Set<Route> -- ultimately, we want {(Mode, Route): Set<Url>}.
		// Some routes (e.g. ping) will be identical between Carleton/StOlaf, but some (/spaces/hours) will not.

		let unrolled_plan: BTreeMap<(Mode, String), BTreeSet<Url>> = targets
			.into_iter()
			.flat_map(|(mode, base_urls)| {
				routes(&mode).map(move |route| ((mode.clone(), route), base_urls.clone()))
			})
			.collect();

		Self()
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let client: Client = ClientBuilder::new()
		.user_agent(concat!(
			env!("CARGO_PKG_NAME"),
			"/",
			env!("CARGO_PKG_VERSION")
		))
		.build()?;

	let targets = test_targets()?;

	let test_plan = TestPlan::generate(targets);

	Ok(())
}
