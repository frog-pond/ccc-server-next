pub struct Proxy {
	client: Option<reqwest::Client>,
}

pub enum ProxyError {}
