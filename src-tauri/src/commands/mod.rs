pub mod analysis;
pub mod exports;
pub mod projects;
pub mod providers;
pub mod rules;
pub mod screenshots;

use uuid::Uuid;

pub fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        projects::list_projects,
        projects::create_project,
        projects::rename_project,
        projects::archive_project,
        projects::delete_project,
        screenshots::list_screenshots,
        screenshots::import_screenshots,
        screenshots::update_screenshot_metadata,
        screenshots::remove_screenshot,
        providers::list_providers,
        providers::save_provider,
        providers::delete_provider,
        providers::test_provider,
        providers::fetch_provider_models,
        analysis::preview_analysis_request,
        analysis::analyze_project,
        rules::get_design_spec,
        rules::update_rule,
        rules::refine_rules,
        exports::list_exports,
        exports::export_design_markdown,
        exports::read_export_markdown,
        exports::reveal_export,
    ]
}

pub type CommandResult<T> = Result<T, String>;

pub fn parse_uuid(value: &str, label: &str) -> CommandResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| format!("{label} is not a valid UUID"))
}

pub fn command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
