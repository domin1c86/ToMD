use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use chrono::Utc;
use design_core::{compile_markdown, DesignSpec, Platform, RuleStatus};
use design_storage::{open_connection, ProjectRepository, ScreenshotRepository, Storage};
use rusqlite::params;
use uuid::Uuid;

const TEST_API_KEY: &str = "sk-test-secret";
const AUTHORIZATION_HEADER: &str = "Authorization: Bearer sk-test-secret";
const BASE64_PREFIX: &str = "data:image/png;base64,";
const PROMPT_TEXT: &str = "Analyze only visible design evidence.";
const PROVIDER_RESPONSE_BODY: &str = "provider raw response body";

#[tokio::test]
async fn screenshot_to_reviewed_markdown_is_local_private_and_repeatable() {
    let temp = tempfile::tempdir().unwrap();
    let app_data_dir = temp.path().join("app-data");
    let storage = Storage::open(&app_data_dir).await.unwrap();
    let network = NetworkProbe::default();

    let project = storage
        .projects()
        .create("Finance", Platform::Mobile)
        .await
        .unwrap();
    assert_eq!(network.calls(), 0);

    let source_path = temp.path().join("dashboard.png");
    fs::write(
        &source_path,
        include_bytes!("../crates/design-storage/tests/fixtures/valid.png"),
    )
    .unwrap();
    let screenshot = storage
        .screenshots()
        .import_screenshot(project.id, &source_path, "Dashboard", "Logged in")
        .await
        .unwrap();
    assert_eq!(network.calls(), 0);

    network.invoke_provider_endpoint();
    let mut spec = provider_success_spec(project.id, screenshot.id);
    accept_all_rules(&mut spec);
    let base_version_id = insert_reviewed_draft(&app_data_dir, project.id, &spec);
    assert_eq!(network.calls(), 1);

    let first_export = write_export(&app_data_dir, project.id, base_version_id, &spec);
    let second_export = write_export(&app_data_dir, project.id, base_version_id, &spec);
    assert_eq!(first_export.contents, second_export.contents);
    assert!(first_export.contents.contains("# Design intent"));
    assert!(!first_export.contents.contains("OriginalBrand"));
    assert_eq!(network.calls(), 1);

    let sensitive_terms = [
        TEST_API_KEY,
        AUTHORIZATION_HEADER,
        BASE64_PREFIX,
        PROMPT_TEXT,
        PROVIDER_RESPONSE_BODY,
    ];
    assert_no_sensitive_terms(&app_data_dir, &sensitive_terms);
    assert_no_sensitive_terms(temp.path(), &sensitive_terms);
}

#[test]
fn invalid_provider_fixture_is_rejected_before_persistence() {
    let invalid: DesignSpec =
        serde_json::from_str(include_str!("fixtures/provider-invalid.json")).unwrap();

    let error = compile_markdown(&invalid).unwrap_err();

    assert!(!error.issues.is_empty());
}

#[derive(Default)]
struct NetworkProbe {
    calls: Arc<AtomicUsize>,
}

impl NetworkProbe {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn invoke_provider_endpoint(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
}

struct ExportedMarkdown {
    contents: String,
}

fn provider_success_spec(project_id: Uuid, screenshot_id: Uuid) -> DesignSpec {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/provider-success.json")).unwrap();
    spec.metadata.project_id = project_id.to_string();
    spec.metadata.source_screenshot_ids = vec![screenshot_id];
    for evidence in &mut spec.evidence {
        evidence.screenshot_id = screenshot_id;
    }
    spec
}

fn accept_all_rules(spec: &mut DesignSpec) {
    for rule in spec
        .intent
        .iter_mut()
        .chain(spec.tokens.iter_mut())
        .chain(spec.layout.iter_mut())
        .chain(spec.components.iter_mut())
        .chain(spec.assets.iter_mut())
        .chain(spec.motion.iter_mut())
        .chain(spec.constraints.iter_mut())
    {
        rule.status = RuleStatus::Accepted;
    }
}

fn insert_reviewed_draft(app_data_dir: &Path, project_id: Uuid, spec: &DesignSpec) -> Uuid {
    let db_path = app_data_dir.join("design-storage.sqlite3");
    let connection = open_connection(&db_path).unwrap();
    let version_id = Uuid::new_v4();
    let provider_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let spec_json = serde_json::to_string(spec).unwrap();
    connection
        .execute(
            "INSERT INTO design_spec_versions (id, project_id, spec_json, provider_id, model, created_at)
             VALUES (?1, ?2, ?3, ?4, 'fixture-model', ?5)",
            params![
                version_id.to_string(),
                project_id.to_string(),
                spec_json,
                provider_id.to_string(),
                now,
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO design_spec_drafts (project_id, base_version_id, spec_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                project_id.to_string(),
                version_id.to_string(),
                serde_json::to_string(spec).unwrap(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();
    version_id
}

fn write_export(
    app_data_dir: &Path,
    project_id: Uuid,
    spec_version_id: Uuid,
    spec: &DesignSpec,
) -> ExportedMarkdown {
    let contents = compile_markdown(spec).unwrap();
    let export_id = Uuid::new_v4();
    let relative_path = format!(
        "exports/{}-DESIGN.md",
        Utc::now().format("%Y%m%dT%H%M%S%.9fZ")
    );
    let destination = app_data_dir
        .join("projects")
        .join(project_id.to_string())
        .join(&relative_path);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    let temp_path = destination.with_extension(format!("md.tmp-{export_id}"));
    fs::write(&temp_path, &contents).unwrap();
    fs::rename(&temp_path, &destination).unwrap();

    let connection = open_connection(&app_data_dir.join("design-storage.sqlite3")).unwrap();
    connection
        .execute(
            "INSERT INTO export_versions (id, project_id, spec_version_id, relative_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                export_id.to_string(),
                project_id.to_string(),
                spec_version_id.to_string(),
                relative_path,
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

    ExportedMarkdown { contents }
}

fn assert_no_sensitive_terms(root: &Path, terms: &[&str]) {
    for file in collect_files(root) {
        let bytes = fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for term in terms {
            assert!(
                !text.contains(term),
                "sensitive term {term:?} leaked into {}",
                file.display()
            );
        }
    }
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(collect_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}
