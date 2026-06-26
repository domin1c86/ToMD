use std::fs;

use chrono::Utc;
use design_core::{
    DesignSpec, Evidence, Platform, Rule, RuleKind, RuleScope, RuleSource, RuleStatus,
};
use design_storage::{ProjectRepository, ScreenshotRepository, Storage, StorageError};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const VALID_PNG: &str = "tests/fixtures/valid.png";
const NOT_AN_IMAGE: &str = "tests/fixtures/not-an-image.txt";

#[tokio::test]
async fn imports_a_copy_with_detected_dimensions_and_hash() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();

    let screenshot = storage
        .screenshots()
        .import_screenshot(
            project.id,
            fixture_path(VALID_PNG).as_path(),
            "Home",
            "Empty state",
        )
        .await
        .unwrap();
    let source_bytes = fs::read(fixture_path(VALID_PNG)).unwrap();
    let destination = temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .join(&screenshot.relative_path);

    assert_eq!(screenshot.project_id, project.id);
    assert_eq!(screenshot.media_type, "image/png");
    assert_eq!(screenshot.width, 1);
    assert_eq!(screenshot.height, 1);
    assert_eq!(screenshot.page_name, "Home");
    assert_eq!(screenshot.scene, "Empty state");
    assert_eq!(screenshot.sort_order, 0);
    assert_eq!(screenshot.sha256, sha256_hex(&source_bytes));
    assert_eq!(
        screenshot.relative_path,
        format!("screenshots/{}.png", screenshot.id)
    );
    assert_eq!(fs::read(destination).unwrap(), source_bytes);
}

#[tokio::test]
async fn rejects_unsupported_or_corrupt_files_without_copying_them() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();

    let error = storage
        .screenshots()
        .import_screenshot(
            project.id,
            fixture_path(NOT_AN_IMAGE).as_path(),
            "Home",
            "Corrupt",
        )
        .await
        .unwrap_err();

    assert!(matches!(error, StorageError::UnsupportedMediaType));
    let connection = Connection::open(temp.path().join("design-storage.sqlite3")).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM screenshots WHERE project_id = ?1",
            params![project.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert!(!temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .join("screenshots")
        .exists());
}

#[tokio::test]
async fn rejects_corrupt_supported_images_without_copying_them() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();
    let corrupt_png = temp.path().join("corrupt.png");
    let mut bytes = fs::read(fixture_path(VALID_PNG)).unwrap();
    bytes[41] ^= 0xFF;
    fs::write(&corrupt_png, bytes).unwrap();

    let error = storage
        .screenshots()
        .import_screenshot(project.id, &corrupt_png, "Home", "Corrupt PNG")
        .await
        .unwrap_err();

    assert!(matches!(error, StorageError::CorruptImage));
    let connection = Connection::open(temp.path().join("design-storage.sqlite3")).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM screenshots WHERE project_id = ?1",
            params![project.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert!(!temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .join("screenshots")
        .exists());
}

#[tokio::test]
async fn rejects_duplicate_hashes_within_the_same_project() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();
    let first = storage
        .screenshots()
        .import_screenshot(
            project.id,
            fixture_path(VALID_PNG).as_path(),
            "Home",
            "Default",
        )
        .await
        .unwrap();

    let error = storage
        .screenshots()
        .import_screenshot(
            project.id,
            fixture_path(VALID_PNG).as_path(),
            "Home",
            "Duplicate",
        )
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        StorageError::DuplicateScreenshot(existing_id) if existing_id == first.id
    ));
    let connection = Connection::open(temp.path().join("design-storage.sqlite3")).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM screenshots WHERE project_id = ?1",
            params![project.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    let duplicate_insert = connection.execute(
        "INSERT INTO screenshots
         (id, project_id, relative_path, sha256, media_type, width, height, page_name, scene, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            Uuid::new_v4().to_string(),
            project.id.to_string(),
            "screenshots/raw-duplicate.png",
            first.sha256,
            "image/png",
            1,
            1,
            "Home",
            "Raw duplicate",
            1,
            Utc::now().to_rfc3339(),
        ],
    );
    assert!(duplicate_insert.is_err());
}

#[tokio::test]
async fn rejects_images_that_exceed_decoded_pixel_budget_without_copying_them() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();
    let huge_png = temp.path().join("huge-header.png");
    fs::write(&huge_png, png_with_dimensions(5_000, 5_000)).unwrap();

    let error = storage
        .screenshots()
        .import_screenshot(project.id, &huge_png, "Home", "Huge")
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        StorageError::ImageTooLarge {
            width: 5_000,
            height: 5_000,
            ..
        }
    ));
    let connection = Connection::open(temp.path().join("design-storage.sqlite3")).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM screenshots WHERE project_id = ?1",
            params![project.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert!(!temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .join("screenshots")
        .exists());
}

#[tokio::test]
async fn removing_a_screenshot_marks_dependent_rules_for_review() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();
    let screenshot = storage
        .screenshots()
        .import_screenshot(
            project.id,
            fixture_path(VALID_PNG).as_path(),
            "Home",
            "Default",
        )
        .await
        .unwrap();
    let evidence_id = Uuid::new_v4();
    let untouched_evidence_id = Uuid::new_v4();
    let accepted_rule_id = Uuid::new_v4();
    let edited_rule_id = Uuid::new_v4();
    let rejected_rule_id = Uuid::new_v4();
    let untouched_rule_id = Uuid::new_v4();
    let mut spec = DesignSpec::empty(project.id.to_string());
    spec.evidence.push(Evidence {
        id: evidence_id,
        screenshot_id: screenshot.id,
        region: None,
        description: "Primary screenshot evidence".to_owned(),
    });
    spec.evidence.push(Evidence {
        id: untouched_evidence_id,
        screenshot_id: Uuid::new_v4(),
        region: None,
        description: "Other screenshot evidence".to_owned(),
    });
    spec.layout.push(rule_with_status(
        accepted_rule_id,
        evidence_id,
        RuleStatus::Accepted,
    ));
    spec.tokens.push(rule_with_status(
        edited_rule_id,
        evidence_id,
        RuleStatus::Edited,
    ));
    spec.assets.push(rule_with_status(
        rejected_rule_id,
        evidence_id,
        RuleStatus::Rejected,
    ));
    spec.components.push(rule_with_status(
        untouched_rule_id,
        untouched_evidence_id,
        RuleStatus::Accepted,
    ));
    insert_draft(temp.path(), project.id, &spec);

    storage
        .screenshots()
        .remove_screenshot(project.id, screenshot.id)
        .await
        .unwrap();

    let updated = read_draft(temp.path(), project.id);
    assert_eq!(
        rule_status(&updated, accepted_rule_id),
        Some(RuleStatus::Pending)
    );
    assert_eq!(
        rule_status(&updated, edited_rule_id),
        Some(RuleStatus::Pending)
    );
    assert_eq!(
        rule_status(&updated, rejected_rule_id),
        Some(RuleStatus::Rejected)
    );
    assert_eq!(
        rule_status(&updated, untouched_rule_id),
        Some(RuleStatus::Accepted)
    );
    assert!(!temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .join(&screenshot.relative_path)
        .exists());
}

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn png_with_dimensions(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x89PNG\r\n\x1A\n");
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    append_png_chunk(&mut bytes, b"IHDR", &ihdr);
    append_png_chunk(
        &mut bytes,
        b"IDAT",
        &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
    append_png_chunk(&mut bytes, b"IEND", &[]);
    bytes
}

fn append_png_chunk(bytes: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
    bytes.extend_from_slice(chunk_type);
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(&crc32(chunk_type, data).to_be_bytes());
}

fn crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in chunk_type.iter().chain(data.iter()) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn rule_with_status(id: Uuid, evidence_id: Uuid, status: RuleStatus) -> Rule {
    Rule {
        id,
        category: "layout".to_owned(),
        statement: "Use observed spacing consistently.".to_owned(),
        kind: RuleKind::Pattern,
        scope: RuleScope::Global,
        value: None,
        evidence_ids: vec![evidence_id],
        confidence: 0.9,
        status,
        source: RuleSource::Model,
    }
}

fn insert_draft(root: &std::path::Path, project_id: Uuid, spec: &DesignSpec) {
    let connection = Connection::open(root.join("design-storage.sqlite3")).unwrap();
    let version_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO design_spec_versions
             (id, project_id, spec_json, provider_id, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                version_id.to_string(),
                project_id.to_string(),
                serde_json::to_string(spec).unwrap(),
                "provider-1",
                "model-1",
                now,
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO design_spec_drafts
             (project_id, base_version_id, spec_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                project_id.to_string(),
                version_id.to_string(),
                serde_json::to_string(spec).unwrap(),
                now,
            ],
        )
        .unwrap();
}

fn read_draft(root: &std::path::Path, project_id: Uuid) -> DesignSpec {
    let connection = Connection::open(root.join("design-storage.sqlite3")).unwrap();
    let json: String = connection
        .query_row(
            "SELECT spec_json FROM design_spec_drafts WHERE project_id = ?1",
            params![project_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    serde_json::from_str(&json).unwrap()
}

fn rule_status(spec: &DesignSpec, rule_id: Uuid) -> Option<RuleStatus> {
    spec.intent
        .iter()
        .chain(spec.tokens.iter())
        .chain(spec.layout.iter())
        .chain(spec.components.iter())
        .chain(spec.assets.iter())
        .chain(spec.motion.iter())
        .chain(spec.constraints.iter())
        .find(|rule| rule.id == rule_id)
        .map(|rule| rule.status)
}
