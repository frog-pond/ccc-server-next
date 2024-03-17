use core::{future::Future, pin::Pin};

use tokio_util::task::TaskTracker;

/// A task that can be run by a ccc-specific worker.
///
/// These are specific types of tasks that are unique to the ccc-server;
/// they are not meant to be used in other contexts.
///
/// # Variants
///
/// - [`WorkerTask::Generic`] - A generic future task that can be spawned.
///   For now, this is the only variant.
///
/// # Examples
///
/// ```
/// use ccc_worker::{Worker, WorkerTask};
/// use tokio::time::Duration;
///
/// let task = WorkerTask::Generic(Box::pin(async {
///     tokio::time::sleep(Duration::from_millis(100)).await;
/// }));
/// ```
pub enum WorkerTask {
	/// A wrapper around any old `async` block that returns ().
	Generic(Pin<Box<dyn Future<Output = ()> + Send>>),
}

/// A worker that can be used to run tasks in the background.
///
/// # Examples
///
/// ```
/// use ccc_worker::{Worker, WorkerTask};
/// use tokio::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let worker = Worker::new();
///     worker.spawn(WorkerTask::Generic(Box::pin(async {
///         tokio::time::sleep(Duration::from_millis(100)).await;
///     })));
///     worker.signal_done_spawning_tasks();
///     worker.wait().await;
/// }
/// ```
pub struct Worker {
	tracker: TaskTracker,
}

impl Default for Worker {
	fn default() -> Self {
		let tracker = TaskTracker::new();
		Self::new()
	}
}

impl Worker {
	pub fn new() -> Self {
		Self {
			..Default::default()
		}
	}

	pub fn spawn(&self, task: WorkerTask) {
		match task {
			WorkerTask::Generic(future) => {
				self.tracker.spawn(future);
			}
		}
	}

	pub fn signal_done_spawning_tasks(&self) {
		self.tracker.close();
	}

	pub async fn wait(&self) {
		self.tracker.wait().await
	}
}
