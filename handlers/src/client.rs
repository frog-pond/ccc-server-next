use once_cell::sync::OnceCell;

static SHARED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

pub fn get_shared_client() -> &'static reqwest::Client {
	SHARED_CLIENT
		.get()
		.expect("request client is not initialized")
}

pub fn initialize() {
	let client = reqwest::Client::builder()
		// Any configuration goes here.
		.build()
		.expect("failed to initialize shared client");

	SHARED_CLIENT
		.set(client)
		.expect("failed to set shared request client");
}
