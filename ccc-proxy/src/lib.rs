pub struct Proxy {
	_client: Option<reqwest::Client>,
}

#[derive(thiserror::Error, Debug)]
pub enum ProxyError {}
