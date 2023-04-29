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

enum Mode {
	StOlaf,
	Carleton,
}

fn get_substitutions(mode: &Mode, token: &str) -> Option<Vec<String>> {
	match token {
		// ":itemId" =>d
		_ => None,
	}
}

fn routes(mode: &Mode) -> impl Iterator<Item = String> {
	let vec: Vec<String> = match mode {
		Mode::StOlaf => include_str!("../STOLAF_ROUTES")
			.lines()
			.map(ToOwned::to_owned)
			.collect(),
		Mode::Carleton => include_str!("../CARLETON_ROUTES")
			.lines()
			.map(ToOwned::to_owned)
			.collect(),
	};

	vec.into_iter()
}

fn make_request(
	client: &Client,
	mode: &Mode,
	source: &str,
	route: &str,
) -> Result<Request, Box<dyn std::error::Error + Send + Sync>> {
	let joined = Url::parse(source)?.join(route)?;
	Ok(client.request(Method::GET, joined).build()?)
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

	let test_targets = vec![
		(
			"http://localhost:3000/api/",
			vec![Mode::Carleton, Mode::StOlaf],
		),
		("https://stolaf.api.frogpond.tech/v1/", vec![Mode::StOlaf]),
		(
			"https://carleton.api.frogpond.tech/v1/",
			vec![Mode::Carleton],
		),
	];

	for (source, modes) in test_targets {
		for mode in modes {
			for route in routes(&mode) {
				if let Ok(request) = make_request(&client, &mode, &source, &route) {
					let req_copy = request.try_clone();

					let response = client.execute(request).await;

					if let Some(rfr) = req_copy {
						println!("{} => {:?}", rfr.url(), response);

						for (header, value) in rfr.headers().iter() {
							println!("{}: {:?}", header, value);
						}
					}
				}
			}
		}
	}

	Ok(())
}
