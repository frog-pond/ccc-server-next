//! Object-safe futures for pipeline stages, resolved by the caller's executor.

use std::{future::Future, pin::Pin};

/// A future that can move between threads on the native executor.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A local future for the Worker executor, including Workers Fetch futures.
#[cfg(target_arch = "wasm32")]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[cfg(test)]
mod tests {
	use std::{
		future::ready,
		task::{Context, Poll, Waker},
	};

	use wasm_bindgen_test::wasm_bindgen_test;

	use super::BoxFuture;

	#[wasm_bindgen_test(unsupported = test)]
	fn boxed_future_preserves_the_target_executor_contract() {
		#[cfg(not(target_arch = "wasm32"))]
		let mut future: BoxFuture<'_, u8> = {
			fn require_send<F: std::future::Future + Send>(future: F) -> F {
				future
			}
			let future: BoxFuture<'_, u8> = Box::pin(ready(42));
			require_send(future)
		};

		#[cfg(target_arch = "wasm32")]
		let mut future: BoxFuture<'_, u8> = {
			let local = std::rc::Rc::new(42);
			Box::pin(async move { ready(*local).await })
		};

		let mut context = Context::from_waker(Waker::noop());
		assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(42));
	}
}
