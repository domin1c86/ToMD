use std::path::PathBuf;

use design_providers::KeyringCredentialStore;
use design_storage::Storage;

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
            http_client: reqwest::Client::new(),
            credential_store: KeyringCredentialStore,
        })
    }
}
