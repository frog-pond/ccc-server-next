//! Compile probes for the draft Trifecta future bound and redirect policy.

#[cfg(target_arch = "wasm32")]
pub mod probes {
	use trifecta::future::BoxFuture;
	use worker::{send::SendFuture, Fetch, Request, RequestInit, RequestRedirect, Response, Url};

	/// Checks the native response and the manual-redirect Fetch future together.
	pub fn fetch(url: &Url) -> BoxFuture<'_, worker::Result<Response>> {
		fn assert_send<T: Send>() {}
		assert_send::<Response>();

		let future = async move {
			let mut init = RequestInit::new();
			init.with_redirect(RequestRedirect::Manual);
			let request = Request::new_with_init(url.as_str(), &init)?;
			Fetch::Request(request).send().await
		};
		#[cfg(worker_experiment_probe = "bare_fetch")]
		let boxed = Box::pin(future);
		#[cfg(not(worker_experiment_probe = "bare_fetch"))]
		let boxed = Box::pin(SendFuture::new(future));
		boxed
	}

	/// Checks the SDK future that reads the response body.
	pub fn read_body(mut response: Response) -> BoxFuture<'static, worker::Result<Vec<u8>>> {
		let future = async move { response.bytes().await };
		#[cfg(worker_experiment_probe = "bare_body")]
		let boxed = Box::pin(future);
		#[cfg(not(worker_experiment_probe = "bare_body"))]
		let boxed = Box::pin(SendFuture::new(future));
		boxed
	}

	/// Checks whether the native reqwest redirect-policy API is present on WASM.
	#[cfg(worker_experiment_probe = "reqwest_redirect")]
	pub fn reqwest_redirect() {
		let _ = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
	}
}

fn main() {}
