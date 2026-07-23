use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::Serialize;

use crate::db::Database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyRecord {
    pub id: i64,
    pub op_kind: String,
    pub output_path: String,
    pub reversible: bool,
    pub created_at: String,
    pub undone_at: Option<String>,
}

pub struct SafetyService {
    exports: PathBuf,
    trash: PathBuf,
}

impl SafetyService {
    pub fn new(exports: PathBuf) -> Result<Self, String> {
        let trash = exports.join(".undo-trash");
        fs::create_dir_all(&trash).map_err(|error| error.to_string())?;
        Ok(Self { exports, trash })
    }

    pub fn list(database: &Database) -> Result<Vec<SafetyRecord>, String> {
        database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, op_kind, after_json, reversible, created_at, undone_at
                     FROM undo_ledger ORDER BY created_at DESC LIMIT 100",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    let after_json: String = row.get(2)?;
                    let output_path = serde_json::from_str::<serde_json::Value>(&after_json)
                        .ok()
                        .and_then(|value| {
                            value
                                .get("output_path")
                                .and_then(serde_json::Value::as_str)
                                .map(ToString::to_string)
                        })
                        .unwrap_or_default();
                    Ok(SafetyRecord {
                        id: row.get(0)?,
                        op_kind: row.get(1)?,
                        output_path,
                        reversible: row.get::<_, i64>(3)? != 0,
                        created_at: row.get(4)?,
                        undone_at: row.get(5)?,
                    })
                })
                .map_err(|error| error.to_string())?;
            rows.map(|row| row.map_err(|error| error.to_string()))
                .collect()
        })
    }

    pub fn undo(&self, database: &Database, id: i64) -> Result<(), String> {
        let record = Self::list(database)?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| "Safety record was not found.".to_string())?;
        if record.undone_at.is_some() {
            return Err("This operation has already been undone.".into());
        }
        if !record.reversible {
            return Err("This operation is not reversible.".into());
        }
        let output = PathBuf::from(record.output_path);
        validate_export_path(&self.exports, &output)?;
        if output.exists() {
            let filename = output
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "The output name is invalid.".to_string())?;
            let destination = self.trash.join(format!("{id}-{filename}"));
            fs::rename(&output, destination).map_err(|error| error.to_string())?;
        }
        database.with_connection(|connection| {
            connection
                .execute(
                    "UPDATE undo_ledger SET undone_at = CURRENT_TIMESTAMP WHERE id = ?1",
                    params![id],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }
}

fn validate_export_path(exports: &Path, output: &Path) -> Result<(), String> {
    let parent = output
        .parent()
        .ok_or_else(|| "The output path has no parent.".to_string())?;
    let canonical_exports = exports.canonicalize().map_err(|error| error.to_string())?;
    let canonical_parent = parent.canonicalize().map_err(|error| error.to_string())?;
    if canonical_parent != canonical_exports {
        return Err("Undo is limited to files created in Tooldeck exports.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_moves_only_a_tooldeck_export() {
        let root = tempfile::tempdir().expect("root");
        let exports = root.path().join("exports");
        fs::create_dir_all(&exports).expect("exports");
        let output = exports.join("result.txt");
        fs::write(&output, "generated").expect("output");
        let database = Database::open_memory().expect("db");
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO undo_ledger(op_kind, before_json, after_json)
                         VALUES ('test', '{}', ?1)",
                        [serde_json::json!({ "output_path": output }).to_string()],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .expect("ledger");
        let service = SafetyService::new(exports).expect("service");
        service.undo(&database, 1).expect("undo");
        assert!(!output.exists());
        assert!(service.trash.join("1-result.txt").exists());
    }

    #[test]
    fn undo_rejects_paths_outside_exports() {
        let root = tempfile::tempdir().expect("root");
        let exports = root.path().join("exports");
        fs::create_dir_all(&exports).expect("exports");
        let outside = root.path().join("original.txt");
        fs::write(&outside, "original").expect("original");
        let error = validate_export_path(&exports, &outside).expect_err("outside rejected");
        assert!(error.contains("limited"));
    }
}
