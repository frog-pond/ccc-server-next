use std::collections::{BTreeMap, BTreeSet, VecDeque};

use reqwest::{Client, ClientBuilder, Url};
use url::ParseError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RouteGroup {
	CccShared,
	StOlafOnly,
	CarletonOnly,
}

fn get_substitutions(mode: &RouteGroup, route: &str, token: &str) -> Vec<&'static str> {
	use RouteGroup::CarletonOnly;

	match (mode, route, token) {
		(_, "food/item/:itemId", ":itemId") => vec!["36221", "13207271", "22061885"],
		(_, "food/menu/:cafeId", ":cafeId") => vec!["261", "262", "263", "35", "36", "24", "458"],
		(_, "food/cafe/:cafeId", ":cafeId") => vec!["261", "262", "263", "35", "36", "24", "458"],

		(CarletonOnly, "convos/upcoming/:id", ":id") => vec![],

		// A panic in this case prompts us to fill out the substitution grid.
		//
		// To do so, add a new match arm for the mode, route, and specific path segment you want to add to.
		_ => {
			panic!("unknown substitution {token} for route {route} in group {mode:?}");
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
		vec![route]
	}
}

fn routes(mode: &RouteGroup) -> impl Iterator<Item = String> {
	let vec: Vec<String> = match mode {
		RouteGroup::CccShared => include_str!("../CCC_ROUTES").lines().map(ToOwned::to_owned),
		RouteGroup::StOlafOnly => include_str!("../STOLAF_ROUTES")
			.lines()
			.map(ToOwned::to_owned),
		RouteGroup::CarletonOnly => include_str!("../CARLETON_ROUTES")
			.lines()
			.map(ToOwned::to_owned),
	}
	.flat_map(|configured_route| substitute(mode, configured_route))
	.collect();

	vec.into_iter()
}

fn test_targets() -> Result<Vec<ServerTarget>, url::ParseError> {
	use RouteGroup::{CarletonOnly, CccShared, StOlafOnly};

	Ok(vec![
		ServerTarget::new(
			Url::parse("http://localhost:3000/api/")?,
			vec![CccShared, CarletonOnly, StOlafOnly],
			TestingTargetType::Candidate,
		),
		ServerTarget::new(
			Url::parse("https://stolaf.api.frogpond.tech/v1/")?,
			vec![CccShared, StOlafOnly],
			TestingTargetType::Reference,
		),
		ServerTarget::new(
			Url::parse("https://carleton.api.frogpond.tech/v1/")?,
			vec![CccShared, CarletonOnly],
			TestingTargetType::Reference,
		),
	])
}

#[derive(Clone, Copy)]
enum TestingTargetType {
	Reference,
	Candidate,
}

#[derive(Clone)]
struct ServerTarget {
	base_url: Url,
	supported_route_groups: Vec<RouteGroup>,
	testing_type: TestingTargetType,
}

impl ServerTarget {
	fn new(
		base_url: Url,
		supported_route_groups: Vec<RouteGroup>,
		testing_type: TestingTargetType,
	) -> Self {
		Self {
			base_url,
			supported_route_groups,
			testing_type,
		}
	}

	fn supported_route_groups(&self) -> &[RouteGroup] {
		&self.supported_route_groups
	}

	fn testing_type(&self) -> &TestingTargetType {
		&self.testing_type
	}

	fn route_url(&self, route: &str) -> Result<Url, ParseError> {
		self.base_url.join(route)
	}
}

struct TestPlan {
	plan: BTreeMap<(RouteGroup, String), BTreeSet<Url>>,
	target_types: BTreeMap<Url, TestingTargetType>,
}

impl TestPlan {
	fn generate(targets: Vec<ServerTarget>) -> Self {
		// "targets" in this case is still structured in terms of (base_url, supported_modes).
		// to turn this into a "plan", we need to normalize in terms of (mode_route, servers_to_test).

		// TODO: "Intern" the servers into a property of the test plan and avoid cloning base urls so repetitively.

		// First, convert targets into {Mode: Set<Url>}:
		let (targets, target_types): (
			BTreeMap<RouteGroup, BTreeSet<Url>>,
			BTreeMap<Url, TestingTargetType>,
		) = targets
			.into_iter()
			.flat_map(
				|ServerTarget {
				   base_url,
				   supported_route_groups,
				   testing_type,
				 }| {
					supported_route_groups
						.into_iter()
						.map(move |supported_mode| (testing_type, supported_mode, base_url.clone()))
				},
			)
			.fold(
				(BTreeMap::default(), BTreeMap::default()),
				|(mut target_map, mut target_type_map), (testing_type, supported_mode, base_url)| {
					target_map
						.entry(supported_mode)
						.or_insert(BTreeSet::default())
						.insert(base_url.clone());

					target_type_map.insert(base_url, testing_type);

					(target_map, target_type_map)
				},
			);

		// "Mode" comes with a Set<Route> -- ultimately, we want {(Mode, Route): Set<Url>}.
		// Some routes (e.g. ping) will be identical between Carleton/StOlaf, but some (/spaces/hours) will not.

		let plan: BTreeMap<(RouteGroup, String), BTreeSet<Url>> = targets
			.into_iter()
			.flat_map(|(mode, base_urls)| {
				routes(&mode).map(move |route| ((mode.clone(), route), base_urls.clone()))
			})
			.collect();

		Self { plan, target_types }
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let _client: Client = ClientBuilder::new()
		.user_agent(concat!(
			env!("CARGO_PKG_NAME"),
			"/",
			env!("CARGO_PKG_VERSION")
		))
		.build()?;

	let targets = test_targets()?;

	let test_plan = TestPlan::generate(targets);

	for ((mode, route), servers) in test_plan.plan {
		println!(
			"testing route \"{route}\" against {} {mode:?} servers",
			servers.len()
		);

		for server in servers {
			let url = server.join(&route)?;

			println!("  {url}");

			// TODO: Fetch all Reference targets
			// TODO: Fetch all Candidate targets

			// TODO: Assertion: Reference targets are identical to each other
			// TODO: Assertion: Candidate matches Reference targets
		}
	}

	Ok(())
}
