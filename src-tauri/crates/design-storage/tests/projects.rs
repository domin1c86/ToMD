use design_core::Platform;
use design_storage::{ProjectRepository, Storage};

#[tokio::test]
async fn create_list_rename_archive_and_delete_project() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage
        .projects()
        .create("Finance app", Platform::Mobile)
        .await
        .unwrap();
    assert!(temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .exists());

    storage
        .projects()
        .rename(project.id, "Money app")
        .await
        .unwrap();
    storage.projects().archive(project.id).await.unwrap();
    assert_eq!(storage.projects().list(false).await.unwrap().len(), 0);

    storage.projects().delete(project.id).await.unwrap();
    assert!(!temp
        .path()
        .join("projects")
        .join(project.id.to_string())
        .exists());
}
