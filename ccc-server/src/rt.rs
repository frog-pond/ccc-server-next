use tokio::{
	io,
	runtime::{Builder, Runtime},
};

pub(super) fn normal() -> io::Result<Runtime> {
	Builder::new_multi_thread().enable_all().build()
}
