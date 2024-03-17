use core::{future::Future, pin::Pin};
use std::time::Duration;

use tokio_util::task::TaskTracker;

enum WorkerTask {
	Generic(Pin<Box<dyn Future<Output = ()> + Send>>),
}

struct WorkerTaskTracker {
	tracker: TaskTracker,
}

impl WorkerTaskTracker {
	fn new() -> Self {
		let tracker = TaskTracker::new();
		Self { tracker }
	}

	fn spawn(&self, task: WorkerTask) {
		match task {
			WorkerTask::Generic(future) => {
				self.tracker.spawn(future);
			}
		}
	}

	async fn wait(&self) {
		self.tracker.close();
		self.tracker.wait().await
	}
}
