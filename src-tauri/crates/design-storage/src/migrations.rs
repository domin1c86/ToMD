use rusqlite::Connection;

/// Ordered, append-only list of schema migrations. `PRAGMA user_version`
/// records how many have been applied, so existing databases only run the
/// migrations added after they were created. Never edit an entry that has
/// shipped; add a new one instead.
const MIGRATIONS: &[&str] = &[INITIAL_MIGRATION];

pub(crate) fn run(connection: &Connection) -> rusqlite::Result<()> {
    run_migrations(connection, MIGRATIONS)
}

fn run_migrations(connection: &Connection, migrations: &[&str]) -> rusqlite::Result<()> {
    let applied: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let applied = usize::try_from(applied).unwrap_or(0);

    for (index, migration) in migrations.iter().enumerate().skip(applied) {
        // The version bump commits atomically with the migration itself.
        let batch = format!(
            "BEGIN IMMEDIATE;\n{}\nPRAGMA user_version = {};\nCOMMIT;",
            migration,
            index + 1
        );
        connection.execute_batch(&batch)?;
    }

    Ok(())
}

const INITIAL_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  platform TEXT NOT NULL,
  archived_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS screenshots (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  media_type TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  page_name TEXT NOT NULL,
  scene TEXT NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_screenshots_project_sha256
ON screenshots(project_id, sha256);

CREATE TABLE IF NOT EXISTS design_spec_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  spec_json TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  model TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS design_spec_drafts (
  project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  base_version_id TEXT NOT NULL REFERENCES design_spec_versions(id),
  spec_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_configs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  base_url TEXT NOT NULL,
  model TEXT NOT NULL,
  credential_ref TEXT NOT NULL,
  capabilities_json TEXT,
  last_tested_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS export_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  spec_version_id TEXT NOT NULL REFERENCES design_spec_versions(id),
  relative_path TEXT NOT NULL,
  created_at TEXT NOT NULL
);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn user_version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn fresh_database_reaches_latest_version() {
        let connection = Connection::open_in_memory().unwrap();

        run(&connection).unwrap();

        assert_eq!(user_version(&connection), MIGRATIONS.len() as i64);
        let tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
    }

    #[test]
    fn run_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();

        run(&connection).unwrap();
        run(&connection).unwrap();

        assert_eq!(user_version(&connection), MIGRATIONS.len() as i64);
    }

    #[test]
    fn pre_versioning_database_upgrades_in_place() {
        let connection = Connection::open_in_memory().unwrap();
        // Databases created before versioning have the schema but user_version 0.
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        assert_eq!(user_version(&connection), 0);

        run(&connection).unwrap();

        assert_eq!(user_version(&connection), MIGRATIONS.len() as i64);
    }

    #[test]
    fn later_migrations_apply_exactly_once() {
        let connection = Connection::open_in_memory().unwrap();
        let migrations = [
            "CREATE TABLE items (id INTEGER PRIMARY KEY);",
            "INSERT INTO items (id) VALUES (1);",
        ];

        run_migrations(&connection, &migrations).unwrap();
        run_migrations(&connection, &migrations).unwrap();

        assert_eq!(user_version(&connection), 2);
        let rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn failed_migration_leaves_version_untouched() {
        let connection = Connection::open_in_memory().unwrap();
        let migrations = [
            "CREATE TABLE items (id INTEGER PRIMARY KEY);",
            "THIS IS NOT SQL;",
        ];

        let error = run_migrations(&connection, &migrations);

        assert!(error.is_err());
        assert_eq!(user_version(&connection), 1);
    }
}
