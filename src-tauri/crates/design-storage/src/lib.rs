mod migrations;
mod projects;
mod screenshots;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use rusqlite::Connection;
use thiserror::Error;

pub use projects::{Project, ProjectRepository, SqliteProjectRepository};
pub use screenshots::{Screenshot, ScreenshotRepository, SqliteScreenshotRepository};

#[derive(Debug, Clone)]
pub struct Storage {
    root: Arc<PathBuf>,
    db_path: Arc<PathBuf>,
}

impl Storage {
    pub async fn open(root: impl AsRef<Path>) -> Result<Self, StorageError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("projects"))?;

        let db_path = root.join("design-storage.sqlite3");
        let connection = open_connection(&db_path)?;
        migrations::run(&connection)?;

        Ok(Self {
            root: Arc::new(root),
            db_path: Arc::new(db_path),
        })
    }

    pub fn projects(&self) -> SqliteProjectRepository {
        SqliteProjectRepository::new(Arc::clone(&self.root), Arc::clone(&self.db_path))
    }

    pub fn screenshots(&self) -> SqliteScreenshotRepository {
        SqliteScreenshotRepository::new(Arc::clone(&self.root), Arc::clone(&self.db_path))
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage I/O failed")]
    Io(#[from] std::io::Error),
    #[error("SQLite operation failed")]
    Sqlite(#[from] rusqlite::Error),
    #[error("background storage task failed")]
    Join(#[from] tokio::task::JoinError),
    #[error("project {0} was not found")]
    ProjectNotFound(uuid::Uuid),
    #[error("stored platform value is not recognized: {0}")]
    InvalidPlatform(String),
    #[error("screenshot media type is not supported")]
    UnsupportedMediaType,
    #[error("screenshot image data is corrupt or unreadable")]
    CorruptImage,
    #[error("screenshot source file exceeds the maximum size")]
    FileTooLarge { size: u64, max: u64 },
    #[error("screenshot dimensions exceed the maximum size")]
    ImageTooLarge { width: u32, height: u32, max: u32 },
    #[error("screenshot duplicates existing screenshot {0}")]
    DuplicateScreenshot(uuid::Uuid),
    #[error("screenshot {0} was not found")]
    ScreenshotNotFound(uuid::Uuid),
    #[error("stored screenshot path is unsafe: {0}")]
    UnsafeScreenshotPath(String),
    #[error("stored design spec JSON is invalid")]
    Json(#[from] serde_json::Error),
    #[error("project cleanup is required at {path}")]
    CleanupRequired {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn open_connection(path: &Path) -> Result<Connection, StorageError> {
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(connection)
}
