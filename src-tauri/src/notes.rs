use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::db::Database;

#[derive(Clone)]
pub struct NotesService {
    vault: Arc<RwLock<PathBuf>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub path: String,
    pub title: String,
    pub preview: String,
    pub tags: Vec<String>,
    pub project_id: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDocument {
    pub path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub project_id: Option<i64>,
    pub updated_at: i64,
}

impl NotesService {
    pub fn new(vault: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&vault).map_err(|error| error.to_string())?;
        Ok(Self {
            vault: Arc::new(RwLock::new(vault)),
        })
    }

    pub fn vault_path(&self) -> Result<PathBuf, String> {
        self.vault
            .read()
            .map(|value| value.clone())
            .map_err(|_| "notes vault lock was poisoned".to_string())
    }

    pub fn switch_vault(&self, database: &Database, vault: PathBuf) -> Result<PathBuf, String> {
        if !vault.is_absolute() {
            return Err("Choose an absolute local folder for the notes vault.".into());
        }
        fs::create_dir_all(&vault).map_err(|error| error.to_string())?;
        let probe = vault.join(".tooldeck-write-check");
        fs::write(&probe, b"local vault")
            .map_err(|error| format!("Tooldeck cannot write to that folder: {error}"))?;
        fs::remove_file(&probe).map_err(|error| error.to_string())?;
        {
            let mut current = self
                .vault
                .write()
                .map_err(|_| "notes vault lock was poisoned".to_string())?;
            *current = vault.clone();
        }
        database.with_connection(|connection| {
            connection
                .execute_batch(
                    "DELETE FROM notes_fts;
                     DELETE FROM notes;
                     DELETE FROM tags;",
                )
                .map_err(|error| error.to_string())?;
            connection
                .execute(
                    "INSERT INTO settings(key, value, updated_at)
                     VALUES ('notes_vault_path', ?1, CURRENT_TIMESTAMP)
                     ON CONFLICT(key) DO UPDATE SET
                        value = excluded.value, updated_at = CURRENT_TIMESTAMP",
                    [vault.to_string_lossy().as_ref()],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })?;
        self.reindex(database)?;
        Ok(vault)
    }

    pub fn reindex(&self, database: &Database) -> Result<(), String> {
        let vault = self.vault_path()?;
        let files = markdown_files(&vault)?;
        database.with_connection(|connection| {
            let mut seen = BTreeSet::new();
            for absolute in files {
                let relative = absolute
                    .strip_prefix(&vault)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .to_string();
                seen.insert(relative.clone());
                let metadata = file_metadata(&absolute)?;
                let indexed = connection
                    .query_row(
                        "SELECT n.updated_at, length(CAST(f.body AS BLOB))
                         FROM notes n
                         JOIN notes_fts f ON f.path = n.path
                         WHERE n.path = ?1",
                        [&relative],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                if indexed == Some((metadata.updated_at, metadata.size)) {
                    continue;
                }
                let Ok(content) = fs::read_to_string(&absolute) else {
                    continue;
                };
                index_note(connection, &relative, &content, metadata.updated_at)?;
            }
            let indexed = {
                let mut statement = connection
                    .prepare("SELECT path FROM notes")
                    .map_err(|error| error.to_string())?;
                let rows = statement
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(|error| error.to_string())?;
                rows.map(|row| row.map_err(|error| error.to_string()))
                    .collect::<Result<Vec<_>, _>>()?
            };
            for missing in indexed.into_iter().filter(|path| !seen.contains(path)) {
                connection
                    .execute("DELETE FROM notes_fts WHERE path = ?1", [&missing])
                    .map_err(|error| error.to_string())?;
                connection
                    .execute("DELETE FROM notes WHERE path = ?1", [&missing])
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }

    pub fn list(&self, database: &Database) -> Result<Vec<NoteSummary>, String> {
        self.reindex(database)?;
        let vault = self.vault_path()?;
        database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT path, title, project_id, updated_at
                     FROM notes ORDER BY updated_at DESC",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            let mut notes = Vec::new();
            for row in rows {
                let (path, title, project_id, updated_at) =
                    row.map_err(|error| error.to_string())?;
                let content = fs::read_to_string(vault.join(&path)).unwrap_or_default();
                notes.push(NoteSummary {
                    path: path.clone(),
                    title,
                    preview: preview_text(&content),
                    tags: tags_for_path(connection, &path)?,
                    project_id,
                    updated_at,
                });
            }
            Ok(notes)
        })
    }

    pub fn get(&self, database: &Database, relative: &str) -> Result<NoteDocument, String> {
        let absolute = self.safe_path(relative)?;
        let content = fs::read_to_string(&absolute).map_err(|error| error.to_string())?;
        let updated_at = file_timestamp(&absolute)?;
        database.with_connection(|connection| {
            index_note(connection, relative, &content, updated_at)?;
            document_from_content(connection, relative, content, updated_at)
        })
    }

    pub fn create(
        &self,
        database: &Database,
        title: &str,
        template: &str,
    ) -> Result<NoteDocument, String> {
        let clean_title = if title.trim().is_empty() {
            "Untitled note"
        } else {
            title.trim()
        };
        let base = slugify(clean_title);
        let relative = unique_note_path(&self.vault_path()?, &base);
        let body = template_body(clean_title, template);
        self.write(&relative, &body)?;
        self.get(database, &relative)
    }

    pub fn daily(&self, database: &Database) -> Result<NoteDocument, String> {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let relative = format!("{date}.md");
        if !self.vault_path()?.join(&relative).exists() {
            let body = format!("# {date}\n\n## Log\n\n- ");
            self.write(&relative, &body)?;
        }
        self.get(database, &relative)
    }

    pub fn save(
        &self,
        database: &Database,
        relative: &str,
        content: &str,
        expected_updated_at: i64,
    ) -> Result<NoteDocument, String> {
        let absolute = self.safe_path(relative)?;
        if absolute.exists() {
            let disk_updated_at = file_timestamp(&absolute)?;
            if expected_updated_at > 0 && disk_updated_at > expected_updated_at + 2 {
                let conflict_name = format!(
                    "{}-conflict-{}.md",
                    Path::new(relative)
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("note"),
                    Local::now().format("%Y%m%d-%H%M%S")
                );
                self.write(&conflict_name, content)?;
                return Err(format!(
                    "This note changed on disk. Your edit was preserved as {conflict_name}."
                ));
            }
        }
        self.write(relative, content)?;
        self.get(database, relative)
    }

    pub fn search(&self, database: &Database, query: &str) -> Result<Vec<NoteSummary>, String> {
        self.reindex(database)?;
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|token| {
                format!(
                    "\"{}\"",
                    token
                        .chars()
                        .filter(|character| character.is_alphanumeric())
                        .collect::<String>()
                )
            })
            .filter(|token| token.len() > 2)
            .collect();
        if tokens.is_empty() {
            return self.list(database);
        }
        let expression = tokens.join(" AND ");
        database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT n.path, n.title, n.project_id, n.updated_at,
                            snippet(notes_fts, 2, '', '', ' … ', 16)
                     FROM notes_fts
                     JOIN notes n ON n.path = notes_fts.path
                     WHERE notes_fts MATCH ?1
                     ORDER BY rank LIMIT 50",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([expression], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            let mut matches = Vec::new();
            for row in rows {
                let (path, title, project_id, updated_at, preview) =
                    row.map_err(|error| error.to_string())?;
                matches.push(NoteSummary {
                    tags: tags_for_path(connection, &path)?,
                    path,
                    title,
                    preview,
                    project_id,
                    updated_at,
                });
            }
            Ok(matches)
        })
    }

    pub fn backlinks(
        &self,
        database: &Database,
        relative: &str,
    ) -> Result<Vec<NoteSummary>, String> {
        let target = self.get(database, relative)?;
        let vault = self.vault_path()?;
        let stem = Path::new(relative)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT DISTINCT n.path, n.title, n.project_id, n.updated_at
                     FROM note_links l JOIN notes n ON n.path = l.source_path
                     WHERE lower(l.target_name) IN (lower(?1), lower(?2))
                     ORDER BY n.updated_at DESC",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map(params![target.title, stem], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            let mut backlinks = Vec::new();
            for row in rows {
                let (path, title, project_id, updated_at) =
                    row.map_err(|error| error.to_string())?;
                let content = fs::read_to_string(vault.join(&path)).unwrap_or_default();
                backlinks.push(NoteSummary {
                    tags: tags_for_path(connection, &path)?,
                    path,
                    title,
                    preview: preview_text(&content),
                    project_id,
                    updated_at,
                });
            }
            Ok(backlinks)
        })
    }

    pub fn link_project(
        &self,
        database: &Database,
        relative: &str,
        project_id: Option<i64>,
    ) -> Result<(), String> {
        self.safe_path(relative)?;
        database.with_connection(|connection| {
            connection
                .execute(
                    "UPDATE notes SET project_id = ?1 WHERE path = ?2",
                    params![project_id, relative],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    pub fn floating_get(&self, database: &Database) -> Result<NoteDocument, String> {
        let relative = "Quick capture.md";
        if !self.vault_path()?.join(relative).exists() {
            self.write(relative, "# Quick capture\n\n")?;
        }
        self.get(database, relative)
    }

    pub fn attach_file(
        &self,
        database: &Database,
        note_relative: &str,
        source: &Path,
    ) -> Result<String, String> {
        let note_path = self.safe_path(note_relative)?;
        if !note_path.is_file() {
            return Err("The note selected for this attachment no longer exists.".into());
        }
        if !source.is_file() {
            return Err("The generated attachment no longer exists.".into());
        }
        let attachments = self.vault_path()?.join("attachments");
        fs::create_dir_all(&attachments).map_err(|error| error.to_string())?;
        let source_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("capture.png");
        let destination_name = format!("{}-{source_name}", now_millis());
        let destination = attachments.join(&destination_name);
        fs::copy(source, &destination).map_err(|error| error.to_string())?;
        let mut content = fs::read_to_string(&note_path).map_err(|error| error.to_string())?;
        content.push_str(&format!(
            "\n\n![{}](attachments/{destination_name})\n",
            source
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Capture")
        ));
        self.write(note_relative, &content)?;
        self.reindex(database)?;
        Ok(format!("attachments/{destination_name}"))
    }

    fn safe_path(&self, relative: &str) -> Result<PathBuf, String> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path.extension().and_then(|value| value.to_str()) != Some("md")
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err("invalid note path".into());
        }
        Ok(self.vault_path()?.join(path))
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), String> {
        let absolute = self.safe_path(relative)?;
        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let temporary = absolute.with_extension("md.tooldeck-tmp");
        fs::write(&temporary, content).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &absolute).map_err(|error| error.to_string())
    }
}

fn document_from_content(
    connection: &Connection,
    relative: &str,
    content: String,
    updated_at: i64,
) -> Result<NoteDocument, String> {
    let (title, project_id) = connection
        .query_row(
            "SELECT title, project_id FROM notes WHERE path = ?1",
            [relative],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .map_err(|error| error.to_string())?;
    Ok(NoteDocument {
        path: relative.to_string(),
        title,
        tags: extract_tags(&content).into_iter().collect(),
        project_id,
        content,
        updated_at,
    })
}

fn index_note(
    connection: &Connection,
    relative: &str,
    content: &str,
    updated_at: i64,
) -> Result<(), String> {
    let title = title_from_content(relative, content);
    connection
        .execute(
            "INSERT INTO notes(path, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                updated_at = excluded.updated_at",
            params![relative, title, updated_at],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute("DELETE FROM notes_fts WHERE path = ?1", [relative])
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO notes_fts(path, title, body) VALUES (?1, ?2, ?3)",
            params![relative, title, content],
        )
        .map_err(|error| error.to_string())?;

    let note_id: i64 = connection
        .query_row("SELECT id FROM notes WHERE path = ?1", [relative], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    connection
        .execute("DELETE FROM note_tags WHERE note_id = ?1", [note_id])
        .map_err(|error| error.to_string())?;
    for tag in extract_tags(content) {
        connection
            .execute("INSERT OR IGNORE INTO tags(name) VALUES (?1)", [&tag])
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO note_tags(note_id, tag_id)
                 SELECT ?1, id FROM tags WHERE name = ?2",
                params![note_id, tag],
            )
            .map_err(|error| error.to_string())?;
    }
    connection
        .execute("DELETE FROM note_links WHERE source_path = ?1", [relative])
        .map_err(|error| error.to_string())?;
    for target in extract_wiki_links(content) {
        connection
            .execute(
                "INSERT OR IGNORE INTO note_links(source_path, target_name)
                 VALUES (?1, ?2)",
                params![relative, target],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn tags_for_path(connection: &Connection, relative: &str) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(
            "SELECT t.name FROM tags t
             JOIN note_tags nt ON nt.tag_id = t.id
             JOIN notes n ON n.id = nt.note_id
             WHERE n.path = ?1 ORDER BY t.name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([relative], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string()))
        .collect()
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                visit(&path, output)?;
            } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
                output.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, &mut files)?;
    Ok(files)
}

fn title_from_content(relative: &str, content: &str) -> String {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            Path::new(relative)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Untitled note")
                .to_string()
        })
}

fn preview_text(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Empty note")
        .chars()
        .take(120)
        .collect()
}

fn extract_tags(content: &str) -> BTreeSet<String> {
    content
        .split_whitespace()
        .filter_map(|word| {
            let tag = word
                .strip_prefix('#')?
                .trim_matches(|character: char| !character.is_alphanumeric() && character != '-');
            (!tag.is_empty()).then(|| tag.to_lowercase())
        })
        .collect()
}

fn extract_wiki_links(content: &str) -> BTreeSet<String> {
    let mut links = BTreeSet::new();
    let mut remaining = content;
    while let Some(start) = remaining.find("[[") {
        let after = &remaining[start + 2..];
        let Some(end) = after.find("]]") else {
            break;
        };
        let target = after[..end].trim();
        if !target.is_empty() {
            links.insert(target.to_string());
        }
        remaining = &after[end + 2..];
    }
    links
}

fn slugify(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("note-{}", now_millis())
    } else {
        slug
    }
}

fn unique_note_path(vault: &Path, base: &str) -> String {
    for suffix in 0..10_000 {
        let name = if suffix == 0 {
            format!("{base}.md")
        } else {
            format!("{base}-{suffix}.md")
        };
        if !vault.join(&name).exists() {
            return name;
        }
    }
    format!("{base}-{}.md", now_millis())
}

fn template_body(title: &str, template: &str) -> String {
    match template {
        "meeting" => format!(
            "# {title}\n\n#meeting\n\n## Attendees\n\n- \n\n## Notes\n\n\n## Actions\n\n- [ ] "
        ),
        "project" => format!(
            "# {title}\n\n#project\n\n## Context\n\n\n## Goals\n\n- \n\n## Decisions\n\n\n## Next steps\n\n- [ ] "
        ),
        "standup" => format!(
            "# {title}\n\n#standup\n\n## Yesterday\n\n- \n\n## Today\n\n- \n\n## Blockers\n\n- None"
        ),
        _ => format!("# {title}\n\n"),
    }
}

fn file_timestamp(path: &Path) -> Result<i64, String> {
    Ok(file_metadata(path)?.updated_at)
}

struct FileMetadata {
    updated_at: i64,
    size: i64,
}

fn file_metadata(path: &Path) -> Result<FileMetadata, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let modified = metadata.modified().map_err(|error| error.to_string())?;
    Ok(FileMetadata {
        updated_at: modified
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis() as i64,
        size: metadata.len() as i64,
    })
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_round_trip_indexes_tags_and_links() {
        let directory = tempfile::tempdir().expect("temporary notes directory");
        let database = Database::open_memory().expect("database");
        let service = NotesService::new(directory.path().to_path_buf()).expect("notes service");
        let created = service
            .create(&database, "Project Alpha", "project")
            .expect("created");
        let saved = service
            .save(
                &database,
                &created.path,
                "# Project Alpha\n\n#client Work with [[Research]].",
                created.updated_at,
            )
            .expect("saved");
        assert!(saved.tags.contains(&"client".to_string()));
        assert_eq!(
            service.search(&database, "Research").expect("search").len(),
            1
        );
    }

    #[test]
    fn external_edit_is_reindexed() {
        let directory = tempfile::tempdir().expect("temporary notes directory");
        let database = Database::open_memory().expect("database");
        let service = NotesService::new(directory.path().to_path_buf()).expect("notes service");
        let created = service
            .create(&database, "Before", "blank")
            .expect("created");
        fs::write(
            directory.path().join(&created.path),
            "# After\n\nExternally changed phrase",
        )
        .expect("external edit");
        let results = service
            .search(&database, "Externally")
            .expect("search results");
        assert_eq!(results[0].title, "After");
    }

    #[test]
    fn unsafe_relative_paths_are_rejected() {
        let directory = tempfile::tempdir().expect("temporary notes directory");
        let service = NotesService::new(directory.path().to_path_buf()).expect("notes service");
        assert!(service.safe_path("../private.md").is_err());
        assert!(service.safe_path("/tmp/private.md").is_err());
        assert!(service.safe_path("binary.png").is_err());
    }

    #[test]
    fn wiki_links_produce_backlinks() {
        let directory = tempfile::tempdir().expect("temporary notes directory");
        let database = Database::open_memory().expect("database");
        let service = NotesService::new(directory.path().to_path_buf()).expect("notes service");
        let target = service
            .create(&database, "Research", "blank")
            .expect("target note");
        service
            .create(&database, "Brief", "blank")
            .and_then(|brief| {
                service.save(
                    &database,
                    &brief.path,
                    "# Brief\n\nSee [[Research]] for the source.",
                    brief.updated_at,
                )
            })
            .expect("source note");
        let backlinks = service
            .backlinks(&database, &target.path)
            .expect("backlinks");
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].title, "Brief");
    }

    #[test]
    fn generated_capture_can_attach_to_a_note() {
        let directory = tempfile::tempdir().expect("temp");
        let notes = NotesService::new(directory.path().join("notes")).expect("notes");
        let database = Database::open_memory().expect("db");
        let note = notes
            .create(&database, "Capture target", "blank")
            .expect("note");
        let capture = directory.path().join("capture.png");
        fs::write(&capture, b"local image bytes").expect("capture");
        let attachment = notes
            .attach_file(&database, &note.path, &capture)
            .expect("attachment");
        assert!(directory.path().join("notes").join(&attachment).is_file());
        let updated = notes.get(&database, &note.path).expect("updated");
        assert!(updated.content.contains(&attachment));
    }

    #[test]
    fn switching_vault_reindexes_without_touching_the_previous_folder() {
        let first = tempfile::tempdir().expect("first");
        let second = tempfile::tempdir().expect("second");
        let database = Database::open_memory().expect("database");
        let service = NotesService::new(first.path().to_path_buf()).expect("notes");
        let original = service
            .create(&database, "Original vault note", "blank")
            .expect("original");
        fs::write(
            second.path().join("Imported.md"),
            "# Imported\n\nSecond vault",
        )
        .expect("fixture");

        service
            .switch_vault(&database, second.path().to_path_buf())
            .expect("switch");
        let notes = service.list(&database).expect("list");

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Imported");
        assert!(first.path().join(original.path).is_file());
    }

    #[test]
    fn reindex_removes_a_note_deleted_outside_tooldeck() {
        let directory = tempfile::tempdir().expect("vault");
        let database = Database::open_memory().expect("database");
        let service = NotesService::new(directory.path().to_path_buf()).expect("notes");
        let note = service
            .create(&database, "Temporary note", "blank")
            .expect("note");
        fs::remove_file(directory.path().join(note.path)).expect("external delete");

        assert!(service.list(&database).expect("list").is_empty());
        let indexed = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, i64>(0))
                    .map_err(|error| error.to_string())
            })
            .expect("count");
        assert_eq!(indexed, 0);
    }
}
