use std::{fs, path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use design_core::Platform;
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::{open_connection, StorageError};

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub platform: Platform,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(async_fn_in_trait)]
pub trait ProjectRepository {
    async fn create(&self, name: &str, platform: Platform) -> Result<Project, StorageError>;
    async fn list(&self, include_archived: bool) -> Result<Vec<Project>, StorageError>;
    async fn get(&self, id: Uuid) -> Result<Project, StorageError>;
    async fn rename(&self, id: Uuid, name: &str) -> Result<Project, StorageError>;
    async fn archive(&self, id: Uuid) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct SqliteProjectRepository {
    root: Arc<PathBuf>,
    db_path: Arc<PathBuf>,
}

impl SqliteProjectRepository {
    pub(crate) fn new(root: Arc<PathBuf>, db_path: Arc<PathBuf>) -> Self {
        Self { root, db_path }
    }

    fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }
}

impl ProjectRepository for SqliteProjectRepository {
    async fn create(&self, name: &str, platform: Platform) -> Result<Project, StorageError> {
        let db_path = Arc::clone(&self.db_path);
        let projects_dir = self.projects_dir();
        let name = name.to_owned();

        tokio::task::spawn_blocking(move || {
            let id = Uuid::new_v4();
            let now = Utc::now();
            let project = Project {
                id,
                name,
                platform,
                archived_at: None,
                created_at: now,
                updated_at: now,
            };

            let project_dir = projects_dir.join(id.to_string());
            fs::create_dir_all(&project_dir)?;

            let connection = open_connection(&db_path)?;
            let insert_result = connection.execute(
                "INSERT INTO projects (id, name, platform, archived_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    project.id.to_string(),
                    project.name,
                    platform_to_str(project.platform),
                    optional_datetime_to_string(project.archived_at),
                    datetime_to_string(project.created_at),
                    datetime_to_string(project.updated_at),
                ],
            );

            if let Err(error) = insert_result {
                let _ = fs::remove_dir_all(project_dir);
                return Err(StorageError::from(error));
            }

            Ok(project)
        })
        .await?
    }

    async fn list(&self, include_archived: bool) -> Result<Vec<Project>, StorageError> {
        let db_path = Arc::clone(&self.db_path);

        tokio::task::spawn_blocking(move || {
            let connection = open_connection(&db_path)?;
            let sql = if include_archived {
                "SELECT id, name, platform, archived_at, created_at, updated_at
                 FROM projects
                 ORDER BY created_at ASC"
            } else {
                "SELECT id, name, platform, archived_at, created_at, updated_at
                 FROM projects
                 WHERE archived_at IS NULL
                 ORDER BY created_at ASC"
            };

            let mut statement = connection.prepare(sql)?;
            let projects = statement
                .query_map([], project_from_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(projects)
        })
        .await?
    }

    async fn get(&self, id: Uuid) -> Result<Project, StorageError> {
        let db_path = Arc::clone(&self.db_path);

        tokio::task::spawn_blocking(move || {
            let connection = open_connection(&db_path)?;
            get_project(&connection, id)
        })
        .await?
    }

    async fn rename(&self, id: Uuid, name: &str) -> Result<Project, StorageError> {
        let db_path = Arc::clone(&self.db_path);
        let name = name.to_owned();

        tokio::task::spawn_blocking(move || {
            let connection = open_connection(&db_path)?;
            let updated_at = Utc::now();
            let changed = connection.execute(
                "UPDATE projects
                 SET name = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![name, datetime_to_string(updated_at), id.to_string()],
            )?;

            if changed == 0 {
                return Err(StorageError::ProjectNotFound(id));
            }

            get_project(&connection, id)
        })
        .await?
    }

    async fn archive(&self, id: Uuid) -> Result<(), StorageError> {
        let db_path = Arc::clone(&self.db_path);

        tokio::task::spawn_blocking(move || {
            let connection = open_connection(&db_path)?;
            let now = datetime_to_string(Utc::now());
            let changed = connection.execute(
                "UPDATE projects
                 SET archived_at = ?1, updated_at = ?1
                 WHERE id = ?2",
                params![now, id.to_string()],
            )?;

            if changed == 0 {
                return Err(StorageError::ProjectNotFound(id));
            }

            Ok(())
        })
        .await?
    }

    async fn delete(&self, id: Uuid) -> Result<(), StorageError> {
        let db_path = Arc::clone(&self.db_path);
        let project_dir = self.projects_dir().join(id.to_string());

        tokio::task::spawn_blocking(move || {
            let mut connection = open_connection(&db_path)?;
            let transaction = connection.transaction()?;
            let changed = transaction.execute(
                "DELETE FROM projects
                 WHERE id = ?1",
                params![id.to_string()],
            )?;

            if changed == 0 {
                return Err(StorageError::ProjectNotFound(id));
            }

            transaction.commit()?;

            if project_dir.exists() {
                fs::remove_dir_all(&project_dir).map_err(|source| {
                    StorageError::CleanupRequired {
                        path: project_dir,
                        source,
                    }
                })?;
            }

            Ok(())
        })
        .await?
    }
}

fn get_project(connection: &Connection, id: Uuid) -> Result<Project, StorageError> {
    connection
        .query_row(
            "SELECT id, name, platform, archived_at, created_at, updated_at
             FROM projects
             WHERE id = ?1",
            params![id.to_string()],
            project_from_row,
        )
        .optional()?
        .ok_or(StorageError::ProjectNotFound(id))
}

fn project_from_row(row: &Row<'_>) -> Result<Project, rusqlite::Error> {
    let id: String = row.get("id")?;
    let platform: String = row.get("platform")?;
    let archived_at: Option<String> = row.get("archived_at")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(Project {
        id: parse_uuid(&id)?,
        name: row.get("name")?,
        platform: platform_from_str(&platform)?,
        archived_at: parse_optional_datetime(archived_at)?,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn platform_to_str(platform: Platform) -> &'static str {
    match platform {
        Platform::Web => "web",
        Platform::Desktop => "desktop",
        Platform::Mobile => "mobile",
        Platform::CrossPlatform => "cross_platform",
    }
}

fn platform_from_str(value: &str) -> Result<Platform, rusqlite::Error> {
    match value {
        "web" => Ok(Platform::Web),
        "desktop" => Ok(Platform::Desktop),
        "mobile" => Ok(Platform::Mobile),
        "cross_platform" => Ok(Platform::CrossPlatform),
        _ => Err(rusqlite::Error::InvalidParameterName(format!(
            "invalid platform: {value}"
        ))),
    }
}

fn datetime_to_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn optional_datetime_to_string(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(datetime_to_string)
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, rusqlite::Error> {
    value
        .parse::<DateTime<Utc>>()
        .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))
}

fn parse_optional_datetime(
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, rusqlite::Error> {
    value.as_deref().map(parse_datetime).transpose()
}

fn parse_uuid(value: &str) -> Result<Uuid, rusqlite::Error> {
    Uuid::parse_str(value).map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))
}
