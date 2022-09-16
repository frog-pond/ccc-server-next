use once_cell::sync::OnceCell;

static SHARED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
