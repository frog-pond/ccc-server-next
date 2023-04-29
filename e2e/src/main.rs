use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RouteGroup {
	StOlaf,
	Carleton,
}

fn get_substitutions(mode: &RouteGroup, route: &str, token: &str) -> Vec<&'static str> {
	use RouteGroup::{Carleton, StOlaf};

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

fn substitute(mode: &RouteGroup, route: String) -> Vec<String> {
	let parts: Vec<&str> = route.split('/').collect();

	// NOTE: We could probably do this with less moving parts if I flagged parts to substitute and then just performed the substitutions based off that math.
	// This approach does decouple the "substitute x with y" from indices pretty well though.
	if parts.iter().any(|part| part.starts_with(':')) {
		let mut good: Vec<String> = Vec::default();

		let mut to_substitute_fully: VecDeque<String> = VecDeque::default();
		to_substitute_fully.push_back(parts.join("/"));

		while let Some(partially_substituted_route) = to_substitute_fully.pop_front() {
			let parts: Vec<&str> = partially_substituted_route.split('/').collect();

			for (idx, part) in parts.iter().enumerate() {
				if !part.starts_with(':') {
					continue;
				}

				let mut this_route: Vec<&str> = parts.clone();

				let substitutions = get_substitutions(mode, &route, part);

				for substitution in substitutions {
					this_route[idx] = substitution;

					let new_string = this_route.join("/");

					if new_string.contains("/:") {
						to_substitute_fully.push_back(new_string);
					} else {
						good.push(new_string);
					}
				}
			}
		}

		good
	} else {
		vec![parts.join("/")]
	}
}

fn routes(mode: &RouteGroup) -> impl Iterator<Item = String> {
	let vec: Vec<String> = match mode {
		RouteGroup::StOlaf => include_str!("../STOLAF_ROUTES")
			.lines()
			.map(ToOwned::to_owned),
		RouteGroup::Carleton => include_str!("../CARLETON_ROUTES")
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

fn test_targets() -> Result<Vec<(url::Url, Vec<RouteGroup>)>, url::ParseError> {
	use RouteGroup::{Carleton, StOlaf};

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
	fn generate(targets: Vec<(Url, Vec<RouteGroup>)>) -> Self {
		// "targets" in this case is still structured in terms of (base_url, supported_modes).
		// to turn this into a "plan", we need to normalize in terms of (mode_route, servers_to_test).

		// First, convert targets into {Mode: Set<Url>}:
		let targets: BTreeMap<RouteGroup, BTreeSet<Url>> = targets
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

		let unrolled_plan: BTreeMap<(RouteGroup, String), BTreeSet<Url>> = targets
			.into_iter()
			.flat_map(|(mode, base_urls)| {
				routes(&mode).map(move |route| ((mode.clone(), route), base_urls.clone()))
			})
			.collect();

		for ((mode, route), servers) in unrolled_plan {
			println!(
				"{:?}, {} -> {:?}",
				mode,
				route,
				servers
					.iter()
					.map(|server| server.as_str())
					.collect::<Vec<_>>()
			);
		}

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
