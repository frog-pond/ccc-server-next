//! Compares responses from the candidate server and reference servers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use reqwest::{
	header::HeaderMap, Client, ClientBuilder, Method, Request, StatusCode, Url, Version,
};
use similar::TextDiff;
use url::ParseError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RouteGroup {
	CccShared,
	StOlafOnly,
	CarletonOnly,
}

fn get_substitutions(
	mode: &RouteGroup,
	route: &str,
	token: &str,
) -> Result<&'static [&'static str], String> {
	use RouteGroup::CarletonOnly;

	match (mode, route, token) {
		(_, "food/item/:itemId", ":itemId") => Ok(&["36221", "13207271", "22061885"]),
		(_, "food/menu/:cafeId" | "food/cafe/:cafeId", ":cafeId") => {
			Ok(&["261", "262", "263", "35", "36", "24", "458"])
		}
		(CarletonOnly, "convos/upcoming/:id", ":id") => Ok(&[]),
		_ => Err(format!(
			"unknown substitution {token} for route {route} in group {mode:?}"
		)),
	}
}

fn substitute(mode: &RouteGroup, route: &str) -> Result<Vec<String>, String> {
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

				let mut this_route = parts.clone();
				for &substitution in get_substitutions(mode, route, part)? {
					*this_route
						.get_mut(idx)
						.ok_or_else(|| format!("missing segment {idx} for route {route}"))? = substitution;
					let new_string = this_route.join("/");

					if new_string.contains("/:") {
						to_substitute_fully.push_back(new_string);
					} else {
						good.push(new_string);
					}
				}
			}
		}

		Ok(good)
	} else {
		Ok(vec![route.to_owned()])
	}
}

fn routes(mode: &RouteGroup) -> Result<Vec<String>, String> {
	let configured_routes = match mode {
		RouteGroup::CccShared => include_str!("../CCC_ROUTES"),
		RouteGroup::StOlafOnly => include_str!("../STOLAF_ROUTES"),
		RouteGroup::CarletonOnly => include_str!("../CARLETON_ROUTES"),
	};
	let mut routes = Vec::new();
	for route in configured_routes.lines() {
		routes.extend(substitute(mode, route)?);
	}
	Ok(routes)
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

#[derive(Clone, Copy, PartialEq)]
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
	fn generate(targets: Vec<ServerTarget>) -> Result<Self, String> {
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

		let mut plan = BTreeMap::new();
		for (mode, base_urls) in targets {
			for route in routes(&mode)? {
				plan.insert((mode.clone(), route), base_urls.clone());
			}
		}

		Ok(Self { plan, target_types })
	}
}

fn make_request(
	client: &Client,
	url: Url,
) -> Result<Request, Box<dyn std::error::Error + Send + Sync>> {
	Ok(client.request(Method::GET, url).build()?)
}

fn header_map_as_string(
	header_map: &HeaderMap,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
	use std::fmt::Write;
	let mut string = String::default();

	let sorted: BTreeMap<String, String> = header_map
		.iter()
		.map(|(header, value)| Ok((header.to_string(), value.to_str()?.to_string())))
		.collect::<Result<_, reqwest::header::ToStrError>>()?;

	for (header, value) in sorted {
		writeln!(&mut string, "{header}: {value}")?;
	}

	Ok(string)
}

fn bytes_to_json_as_string(body: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
	let result = match serde_json::from_str::<serde_json::Value>(body) {
		Ok(value) => serde_json::to_string_pretty(&value)?,
		Err(_) => body.to_string(),
	};

	Ok(result)
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

	let test_plan = TestPlan::generate(targets)?;
	type Response = Result<(Version, StatusCode, HeaderMap, String), reqwest::Error>;

	for ((mode, route), servers) in test_plan.plan {
		println!(
			"testing route \"{route}\" against {} {mode:?} servers",
			servers.len()
		);

		let (candidates, references): (Vec<_>, Vec<_>) = servers
			.into_iter()
			.partition(|s| test_plan.target_types.get(s) == Some(&TestingTargetType::Candidate));

		println!("  references:");

		let mut reference_results: BTreeMap<String, BTreeMap<Url, Response>> = BTreeMap::default();

		for reference in references {
			let url = reference.join(&route)?;

			println!("    {url}");

			if let Ok(request) = make_request(&client, url.clone()) {
				let response = client.execute(request).await;

				match response {
					Ok(response) => {
						let version = response.version();
						let status = response.status();
						let headers = response.headers().to_owned();
						let body = response.text().await?;

						reference_results
							.entry(route.clone())
							.or_default()
							.insert(url.clone(), Ok((version, status, headers, body)));
					}
					Err(e) => {
						reference_results
							.entry(route.clone())
							.or_default()
							.insert(url.clone(), Err(e));
					}
				}
			}
		}

		// TODO: Assertion: Reference targets are identical to each other

		// dbg!(reference_results);

		println!("  candidates:");

		let mut candidate_results: BTreeMap<String, BTreeMap<Url, Response>> = BTreeMap::default();

		for candidate in candidates {
			let url = candidate.join(&route)?;

			println!("    {url}");

			if let Ok(request) = make_request(&client, url.clone()) {
				let response = client.execute(request).await;

				match response {
					Ok(response) => {
						let version = response.version();
						let status = response.status();
						let headers = response.headers().to_owned();
						let body = response.text().await?;

						candidate_results
							.entry(route.clone())
							.or_default()
							.insert(url.clone(), Ok((version, status, headers, body)));
					}
					Err(e) => {
						candidate_results
							.entry(route.clone())
							.or_default()
							.insert(url.clone(), Err(e));
					}
				}
			}

			// TODO: Assertion: Candidate matches Reference targets

			for candidate_result in candidate_results
				.get(&route)
				.ok_or_else(|| format!("no candidate results for route {route}"))?
				.values()
			{
				for reference_result in reference_results
					.get(&route)
					.ok_or_else(|| format!("no reference results for route {route}"))?
					.values()
				{
					let candidate_result = &candidate_result;

					let reference_result = &reference_result;

					match (candidate_result, reference_result) {
						(
							Ok((candidate_version, candidate_status, candidate_headers, candidate_bytes)),
							Ok((reference_version, reference_status, reference_headers, reference_bytes)),
						) => {
							let reference_headers_string = header_map_as_string(reference_headers)?;
							let candidate_headers_string = header_map_as_string(candidate_headers)?;

							let reference_body_string = bytes_to_json_as_string(reference_bytes)?;
							let candidate_body_string = bytes_to_json_as_string(candidate_bytes)?;

							let reference_string = format!(
								"{reference_version:?} {reference_status}\n{reference_headers_string}\n\n{reference_body_string}",
							);
							let candidate_string =
								format!("{candidate_version:?} {candidate_status}\n{candidate_headers_string}\n\n{candidate_body_string}");

							if reference_string == candidate_string {
								println!("identical");
							} else {
								let diff = TextDiff::from_lines(&reference_string, &candidate_string);

								for change in diff.iter_all_changes() {
									let sign = match change.tag() {
										similar::ChangeTag::Delete => "-",
										similar::ChangeTag::Insert => "+",
										similar::ChangeTag::Equal => " ",
									};
									print!("{sign}{change}");
								}
							}
						}
						(Err(error), _) | (_, Err(error)) => {
							return Err(format!("request for route {route} failed: {error}").into());
						}
					}
				}
			}
		}
	}

	Ok(())
}
