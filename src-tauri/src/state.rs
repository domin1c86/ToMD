use std::{path::PathBuf, time::Duration};

use design_providers::KeyringCredentialStore;
use design_storage::Storage;

/// Upper bound for a full analysis round-trip; large multi-image requests
/// against slow providers can legitimately take minutes.
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub app_data_dir: PathBuf,
    pub db_path: PathBuf,
    pub http_client: reqwest::Client,
    pub credential_store: KeyringCredentialStore,
}

impl AppState {
    pub async fn open(app_data_dir: PathBuf) -> Result<Self, design_storage::StorageError> {
        let storage = Storage::open(&app_data_dir).await?;
        let db_path = app_data_dir.join("design-storage.sqlite3");

        Ok(Self {
            storage,
            app_data_dir,
            db_path,
            http_client: reqwest::Client::builder()
                .connect_timeout(HTTP_CONNECT_TIMEOUT)
                .timeout(HTTP_REQUEST_TIMEOUT)
                .build()
                .expect("failed to build HTTP client"),
            credential_store: KeyringCredentialStore,
        })
    }
}
