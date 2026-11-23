//! The Durable Object input gate binding missing from workers-rs 0.8.5.

use std::{cell::Cell, future::Future, panic::AssertUnwindSafe, rc::Rc};

use worker::{
	js_sys::{futures::future_to_promise, Function, Promise},
	wasm_bindgen::{
		self,
		prelude::{wasm_bindgen, Closure, JsValue},
		JsCast,
	},
	wasm_bindgen_futures::JsFuture,
	State,
};

#[wasm_bindgen]
extern "C" {
	/// A handle to the same JavaScript Durable Object state as [`State`].
	pub(super) type InputGate;

	#[wasm_bindgen(method, catch, js_name = blockConcurrencyWhile)]
	fn block(this: &InputGate, callback: &Function) -> Result<Promise, JsValue>;
}

impl InputGate {
	/// Retains the SDK state for storage and gives the same object an input gate.
	pub(super) fn from_state(state: State) -> (State, Self) {
		let raw = state._inner();
		let gate = raw.clone().unchecked_into();
		(State::from(raw), gate)
	}

	/// Runs the operation while the runtime holds new object events. Any routine
	/// storage failure is carried as the Rust return value.
	///
	/// # Errors
	///
	/// Returns the JS exception if the runtime rejects the gate or its promise,
	/// or an error if the gate resolves without running the callback.
	pub(super) async fn run<T: 'static>(
		&self,
		future: impl Future<Output = T> + 'static,
	) -> worker::Result<T> {
		let output = Rc::new(Cell::new(None));
		let slot = Rc::clone(&output);
		let callback = Closure::once(AssertUnwindSafe(move || {
			future_to_promise(AssertUnwindSafe(async move {
				slot.set(Some(future.await));
				Ok(JsValue::UNDEFINED)
			}))
		}));
		let promise = self.block(callback.as_ref().unchecked_ref())?;
		JsFuture::from(promise).await?;
		output
			.take()
			.ok_or_else(|| worker::Error::RustError("input gate callback did not run".into()))
	}
}
