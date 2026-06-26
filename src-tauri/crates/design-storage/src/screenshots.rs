use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use design_core::{DesignSpec, Rule, RuleStatus};
use image::{ImageFormat, ImageReader};
use rusqlite::{params, ErrorCode, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{open_connection, StorageError};

const MAX_SOURCE_BYTES: u64 = 25 * 1024 * 1024;
const MAX_DIMENSION: u32 = 16_384;
const MAX_DECODED_PIXELS: u64 = 16_777_216;

#[derive(Debug, Clone, PartialEq)]
pub struct Screenshot {
    pub id: Uuid,
    pub project_id: Uuid,
    pub relative_path: String,
    pub sha256: String,
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub page_name: String,
    pub scene: String,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
}

#[allow(async_fn_in_trait)]
pub trait ScreenshotRepository {
    async fn import_screenshot(
        &self,
        project_id: Uuid,
        source: &Path,
        page_name: &str,
        scene: &str,
    ) -> Result<Screenshot, StorageError>;

    async fn remove_screenshot(
        &self,
        project_id: Uuid,
        screenshot_id: Uuid,
    ) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct SqliteScreenshotRepository {
    root: Arc<PathBuf>,
    db_path: Arc<PathBuf>,
}

impl SqliteScreenshotRepository {
    pub(crate) fn new(root: Arc<PathBuf>, db_path: Arc<PathBuf>) -> Self {
        Self { root, db_path }
    }

    fn project_dir(&self, project_id: Uuid) -> PathBuf {
        self.root.join("projects").join(project_id.to_string())
    }
}

impl ScreenshotRepository for SqliteScreenshotRepository {
    async fn import_screenshot(
        &self,
        project_id: Uuid,
        source: &Path,
        page_name: &str,
        scene: &str,
    ) -> Result<Screenshot, StorageError> {
        let db_path = Arc::clone(&self.db_path);
        let project_dir = self.project_dir(project_id);
        let source = source.to_path_buf();
        let page_name = page_name.to_owned();
        let scene = scene.to_owned();

        tokio::task::spawn_blocking(move || {
            let metadata = fs::metadata(&source)?;
            if metadata.len() > MAX_SOURCE_BYTES {
                return Err(StorageError::FileTooLarge {
                    size: metadata.len(),
                    max: MAX_SOURCE_BYTES,
                });
            }

            let bytes = fs::read(&source)?;
            let image_metadata = detect_image_metadata(&bytes)?;

            let sha256 = hex_sha256(&bytes);
            let mut connection = open_connection(&db_path)?;
            let transaction = connection.transaction()?;
            ensure_project_exists(&transaction, project_id)?;

            if let Some(existing_id) =
                find_duplicate(&transaction, project_id, sha256.as_str())?
            {
                return Err(StorageError::DuplicateScreenshot(existing_id));
            }
            let sort_order = next_sort_order(&transaction, project_id)?;

            let screenshot_id = Uuid::new_v4();
            let relative_path = format!("screenshots/{screenshot_id}.{}", image_metadata.extension);
            let screenshots_dir = project_dir.join("screenshots");
            fs::create_dir_all(&screenshots_dir)?;
            let temp_path = screenshots_dir.join(format!(".{screenshot_id}.tmp"));
            let final_path = project_dir.join(&relative_path);
            fs::write(&temp_path, &bytes)?;

            let created_at = Utc::now();
            let screenshot = Screenshot {
                id: screenshot_id,
                project_id,
                relative_path,
                sha256,
                media_type: image_metadata.media_type.to_owned(),
                width: image_metadata.width,
                height: image_metadata.height,
                page_name,
                scene,
                sort_order,
                created_at,
            };

            let insert_result = transaction.execute(
                "INSERT INTO screenshots
                 (id, project_id, relative_path, sha256, media_type, width, height, page_name, scene, sort_order, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    screenshot.id.to_string(),
                    screenshot.project_id.to_string(),
                    screenshot.relative_path,
                    screenshot.sha256,
                    screenshot.media_type,
                    screenshot.width,
                    screenshot.height,
                    screenshot.page_name,
                    screenshot.scene,
                    screenshot.sort_order,
                    datetime_to_string(screenshot.created_at),
                ],
            );

            if let Err(error) = insert_result {
                let _ = fs::remove_file(&temp_path);
                if is_constraint_violation(&error) {
                    if let Some(existing_id) =
                        find_duplicate(&transaction, project_id, screenshot.sha256.as_str())?
                    {
                        return Err(StorageError::DuplicateScreenshot(existing_id));
                    }
                }
                return Err(StorageError::from(error));
            }

            if let Err(error) = fs::rename(&temp_path, &final_path) {
                let _ = fs::remove_file(&temp_path);
                return Err(StorageError::from(error));
            }

            if let Err(error) = transaction.commit() {
                let _ = fs::remove_file(&final_path);
                return Err(StorageError::from(error));
            }

            Ok(screenshot)
        })
        .await?
    }

    async fn remove_screenshot(
        &self,
        project_id: Uuid,
        screenshot_id: Uuid,
    ) -> Result<(), StorageError> {
        let db_path = Arc::clone(&self.db_path);
        let project_dir = self.project_dir(project_id);

        tokio::task::spawn_blocking(move || {
            let mut connection = open_connection(&db_path)?;
            let transaction = connection.transaction()?;
            let relative_path: String = transaction
                .query_row(
                    "SELECT relative_path
                     FROM screenshots
                     WHERE id = ?1 AND project_id = ?2",
                    params![screenshot_id.to_string(), project_id.to_string()],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(StorageError::ScreenshotNotFound(screenshot_id))?;

            mark_dependent_rules_pending(&transaction, project_id, screenshot_id)?;

            transaction.execute(
                "DELETE FROM screenshots
                 WHERE id = ?1 AND project_id = ?2",
                params![screenshot_id.to_string(), project_id.to_string()],
            )?;
            transaction.commit()?;

            let file_path = project_dir.join(relative_path);
            if file_path.exists() {
                fs::remove_file(&file_path).map_err(|source| StorageError::CleanupRequired {
                    path: file_path,
                    source,
                })?;
            }

            Ok(())
        })
        .await?
    }
}

struct ImageMetadata {
    extension: &'static str,
    media_type: &'static str,
    width: u32,
    height: u32,
}

fn detect_image_metadata(bytes: &[u8]) -> Result<ImageMetadata, StorageError> {
    let format = image::guess_format(bytes).map_err(|_| StorageError::UnsupportedMediaType)?;
    let (extension, media_type) = match format {
        ImageFormat::Png => ("png", "image/png"),
        ImageFormat::Jpeg => ("jpg", "image/jpeg"),
        ImageFormat::WebP => ("webp", "image/webp"),
        _ => return Err(StorageError::UnsupportedMediaType),
    };
    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| StorageError::CorruptImage)?;
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(StorageError::ImageTooLarge {
            width,
            height,
            max: MAX_DIMENSION,
        });
    }
    if u64::from(width) * u64::from(height) > MAX_DECODED_PIXELS {
        return Err(StorageError::ImageTooLarge {
            width,
            height,
            max: MAX_DIMENSION,
        });
    }

    let decoded = ImageReader::with_format(Cursor::new(bytes), format)
        .decode()
        .map_err(|_| StorageError::CorruptImage)?;

    Ok(ImageMetadata {
        extension,
        media_type,
        width: decoded.width(),
        height: decoded.height(),
    })
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn ensure_project_exists(
    transaction: &Transaction<'_>,
    project_id: Uuid,
) -> Result<(), StorageError> {
    let exists: Option<String> = transaction
        .query_row(
            "SELECT id FROM projects WHERE id = ?1",
            params![project_id.to_string()],
            |row| row.get(0),
        )
        .optional()?;

    if exists.is_some() {
        Ok(())
    } else {
        Err(StorageError::ProjectNotFound(project_id))
    }
}

fn find_duplicate(
    transaction: &Transaction<'_>,
    project_id: Uuid,
    sha256: &str,
) -> Result<Option<Uuid>, StorageError> {
    transaction
        .query_row(
            "SELECT id
             FROM screenshots
             WHERE project_id = ?1 AND sha256 = ?2
             LIMIT 1",
            params![project_id.to_string(), sha256],
            |row| {
                let id: String = row.get(0)?;
                parse_uuid(&id)
            },
        )
        .optional()
        .map_err(StorageError::from)
}

fn next_sort_order(transaction: &Transaction<'_>, project_id: Uuid) -> Result<i64, StorageError> {
    let max_sort_order: Option<i64> = transaction.query_row(
        "SELECT MAX(sort_order)
         FROM screenshots
         WHERE project_id = ?1",
        params![project_id.to_string()],
        |row| row.get(0),
    )?;
    Ok(max_sort_order.map_or(0, |value| value + 1))
}

fn is_constraint_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::ConstraintViolation
    )
}

fn mark_dependent_rules_pending(
    transaction: &Transaction<'_>,
    project_id: Uuid,
    screenshot_id: Uuid,
) -> Result<(), StorageError> {
    let draft_json: Option<String> = transaction
        .query_row(
            "SELECT spec_json
             FROM design_spec_drafts
             WHERE project_id = ?1",
            params![project_id.to_string()],
            |row| row.get(0),
        )
        .optional()?;

    let Some(draft_json) = draft_json else {
        return Ok(());
    };

    let mut spec: DesignSpec = serde_json::from_str(&draft_json)?;
    let stale_evidence_ids = spec
        .evidence
        .iter()
        .filter(|evidence| evidence.screenshot_id == screenshot_id)
        .map(|evidence| evidence.id)
        .collect::<Vec<_>>();

    if stale_evidence_ids.is_empty() {
        return Ok(());
    }

    for rule in all_rules_mut(&mut spec) {
        if rule
            .evidence_ids
            .iter()
            .any(|evidence_id| stale_evidence_ids.contains(evidence_id))
            && matches!(rule.status, RuleStatus::Accepted | RuleStatus::Edited)
        {
            rule.status = RuleStatus::Pending;
        }
    }

    transaction.execute(
        "UPDATE design_spec_drafts
         SET spec_json = ?1, updated_at = ?2
         WHERE project_id = ?3",
        params![
            serde_json::to_string(&spec)?,
            datetime_to_string(Utc::now()),
            project_id.to_string(),
        ],
    )?;

    Ok(())
}

fn all_rules_mut(spec: &mut DesignSpec) -> impl Iterator<Item = &mut Rule> {
    spec.intent
        .iter_mut()
        .chain(spec.tokens.iter_mut())
        .chain(spec.layout.iter_mut())
        .chain(spec.components.iter_mut())
        .chain(spec.assets.iter_mut())
        .chain(spec.motion.iter_mut())
        .chain(spec.constraints.iter_mut())
}

fn datetime_to_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn parse_uuid(value: &str) -> Result<Uuid, rusqlite::Error> {
    Uuid::parse_str(value).map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))
}
