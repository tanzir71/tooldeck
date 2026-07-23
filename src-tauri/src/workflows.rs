use std::path::Path;

use chrono::Local;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::Database;
use crate::notes::NotesService;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub icon: &'static str,
    pub input_hint: &'static str,
}

pub const DEFINITIONS: &[WorkflowDefinition] = &[
    WorkflowDefinition {
        id: "workflow-billable-document",
        name: "Billable → document",
        summary: "Turn billable time into an invoice, timesheet, or activity report.",
        icon: "file-output",
        input_hint: "Uses completed billable entries.",
    },
    WorkflowDefinition {
        id: "workflow-documentation",
        name: "Documentation companion",
        summary: "Style a project note into a clean project document or handoff PDF.",
        icon: "square-pen",
        input_hint: "Uses a selected note or the most recent note.",
    },
    WorkflowDefinition {
        id: "workflow-research",
        name: "Research companion",
        summary: "Capture an extracted or OCR excerpt in a source-linked local note.",
        icon: "captions",
        input_hint: "Needs a source excerpt.",
    },
    WorkflowDefinition {
        id: "workflow-context-switch",
        name: "Context switcher",
        summary: "Switch the active project context for notes, time, and capture.",
        icon: "arrow-left-right",
        input_hint: "Needs a local project.",
    },
    WorkflowDefinition {
        id: "workflow-standup",
        name: "Meeting / standup log",
        summary: "Append a timestamped bullet to today’s daily note.",
        icon: "clock",
        input_hint: "Needs one short update.",
    },
    WorkflowDefinition {
        id: "workflow-capture-attach",
        name: "Capture → annotate → attach",
        summary: "Open region capture with a target note already in context.",
        icon: "camera",
        input_hint: "Needs a note path.",
    },
    WorkflowDefinition {
        id: "workflow-media-inline",
        name: "Media micro-task inline",
        summary: "Turn a short clip into a capped GIF and attach it to a note.",
        icon: "play",
        input_hint: "Needs a local video.",
    },
];

pub fn install_defaults(database: &Database) -> Result<(), String> {
    database.with_connection(|connection| {
        for definition in DEFINITIONS {
            connection
                .execute(
                    "INSERT INTO workflows(id, name, description, steps_json)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        steps_json = excluded.steps_json,
                        updated_at = CURRENT_TIMESTAMP",
                    rusqlite::params![
                        definition.id,
                        definition.name,
                        definition.summary,
                        json!({
                            "engineVersion": 1,
                            "inputHint": definition.input_hint
                        })
                        .to_string()
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
}

#[derive(Debug, Clone)]
pub enum WorkflowPlan {
    Job {
        kind: String,
        input: Value,
        message: String,
    },
    NextTool {
        tool_id: String,
        prefill: Value,
        message: String,
    },
    Done {
        message: String,
        context: Value,
    },
    Degraded {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunResult {
    pub status: String,
    pub message: String,
    pub job_id: Option<String>,
    pub next_tool_id: Option<String>,
    pub prefill: Option<Value>,
    pub context: Option<Value>,
}

pub fn plan(
    database: &Database,
    notes: &NotesService,
    id: &str,
    input: &Value,
) -> Result<WorkflowPlan, String> {
    match id {
        "workflow-billable-document" => billable_document(database, input),
        "workflow-documentation" => documentation(database, notes, input),
        "workflow-research" => research(database, notes, input),
        "workflow-context-switch" => context_switch(database, input),
        "workflow-standup" => standup(database, notes, input),
        "workflow-capture-attach" => capture_attach(database, notes, input),
        "workflow-media-inline" => media_inline(input),
        _ => Err(format!("Unknown workflow: {id}")),
    }
}

fn billable_document(database: &Database, input: &Value) -> Result<WorkflowPlan, String> {
    let summary = database.with_connection(|connection| {
        connection
            .query_row(
                "SELECT p.name,
                        COALESCE(SUM(e.stopped_at - e.started_at), 0),
                        COALESCE(MAX(e.rate_snapshot), 0),
                        COUNT(e.id)
                 FROM time_entries e
                 LEFT JOIN projects p ON p.id = e.project_id
                 WHERE e.billable = 1 AND e.stopped_at IS NOT NULL",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .map_err(|error| error.to_string())
    })?;
    if summary.3 == 0 {
        return Ok(WorkflowPlan::Degraded {
            message: "No completed billable entries yet. Track or add billable time, then run this workflow again.".into(),
        });
    }
    let template = input
        .get("template")
        .and_then(Value::as_str)
        .unwrap_or("invoice");
    let project = summary.0.unwrap_or_else(|| "Billable work".into());
    let hours = summary.1 as f64 / 3600.0;
    Ok(WorkflowPlan::Job {
        kind: "export-document".into(),
        input: json!({
            "template": template,
            "title": format!("{project} · {}", Local::now().format("%Y-%m-%d")),
            "client": project,
            "body": format!("{} completed billable entries", summary.3),
            "hours": hours,
            "rate": summary.2,
            "tax": input.get("tax").and_then(Value::as_f64).unwrap_or(0.0)
        }),
        message: format!("Prepared {hours:.2} billable hours for export."),
    })
}

fn documentation(
    database: &Database,
    notes: &NotesService,
    input: &Value,
) -> Result<WorkflowPlan, String> {
    let requested = input
        .get("note_path")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string);
    let note_path = if requested.is_some() {
        requested
    } else {
        database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT path FROM notes ORDER BY updated_at DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())
        })?
    };
    let Some(note_path) = note_path else {
        return Ok(WorkflowPlan::Degraded {
            message: "There isn’t a note to export yet. Create a project note, then return here."
                .into(),
        });
    };
    let note = notes.get(database, &note_path)?;
    Ok(WorkflowPlan::Job {
        kind: "export-document".into(),
        input: json!({
            "template": "project-document",
            "title": note.title,
            "client": input.get("project").and_then(Value::as_str).unwrap_or("Local project"),
            "body": note.content
        }),
        message: format!("Prepared “{}” as a project document.", note.title),
    })
}

fn research(
    database: &Database,
    notes: &NotesService,
    input: &Value,
) -> Result<WorkflowPlan, String> {
    let Some(excerpt) = input
        .get("source_text")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(WorkflowPlan::Degraded {
            message: "Paste an extracted or OCR excerpt first. Nothing was changed.".into(),
        });
    };
    let source = input
        .get("source_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Local source");
    let mut note = notes.create(database, &format!("Research · {source}"), "research")?;
    let content = format!(
        "# Research · {source}\n\nSource: `{source}`\n\n## Excerpt\n\n> {}\n\n## Notes\n\n",
        excerpt.replace('\n', "\n> ")
    );
    note = notes.save(database, &note.path, &content, note.updated_at)?;
    Ok(WorkflowPlan::Done {
        message: format!("Captured the excerpt in “{}”.", note.title),
        context: json!({ "notePath": note.path }),
    })
}

fn context_switch(database: &Database, input: &Value) -> Result<WorkflowPlan, String> {
    let Some(project_id) = input.get("project_id").and_then(Value::as_i64) else {
        return Ok(WorkflowPlan::Degraded {
            message: "Choose a project before switching context.".into(),
        });
    };
    let context = database.with_connection(|connection| {
        let project = connection
            .query_row(
                "SELECT name, color FROM projects WHERE id = ?1 AND archived = 0",
                [project_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((name, color)) = project else {
            return Ok(None);
        };
        let note_path = connection
            .query_row(
                "SELECT path FROM notes WHERE project_id = ?1 ORDER BY updated_at DESC LIMIT 1",
                [project_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO settings(key, value, updated_at)
                 VALUES ('active_project_id', ?1, CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value, updated_at = CURRENT_TIMESTAMP",
                [project_id.to_string()],
            )
            .map_err(|error| error.to_string())?;
        Ok(Some(json!({
            "projectId": project_id,
            "projectName": name,
            "color": color,
            "notePath": note_path
        })))
    })?;
    match context {
        Some(context) => Ok(WorkflowPlan::Done {
            message: format!(
                "Switched to {}. New timers and captures can use this context.",
                context["projectName"].as_str().unwrap_or("the project")
            ),
            context,
        }),
        None => Ok(WorkflowPlan::Degraded {
            message: "That project is unavailable or archived. Nothing was changed.".into(),
        }),
    }
}

fn standup(
    database: &Database,
    notes: &NotesService,
    input: &Value,
) -> Result<WorkflowPlan, String> {
    let Some(bullet) = input
        .get("bullet")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(WorkflowPlan::Degraded {
            message: "Write one short update before adding it to today’s log.".into(),
        });
    };
    let note = notes.daily(database)?;
    let content = format!(
        "{}\n- {} — {}",
        note.content.trim_end(),
        Local::now().format("%H:%M"),
        bullet.trim()
    );
    let note = notes.save(database, &note.path, &content, note.updated_at)?;
    Ok(WorkflowPlan::Done {
        message: "Added a timestamped update to today’s daily log.".into(),
        context: json!({ "notePath": note.path }),
    })
}

fn capture_attach(
    database: &Database,
    notes: &NotesService,
    input: &Value,
) -> Result<WorkflowPlan, String> {
    let Some(note_path) = input
        .get("note_path")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(WorkflowPlan::Degraded {
            message: "Choose a target note before capturing. Nothing was changed.".into(),
        });
    };
    if notes.get(database, note_path).is_err() {
        return Ok(WorkflowPlan::Degraded {
            message: "That note could not be found in the local vault.".into(),
        });
    }
    Ok(WorkflowPlan::NextTool {
        tool_id: "screenshot-capture".into(),
        prefill: json!({ "note_path": note_path }),
        message: "Target note is ready. Choose a region in the capture tool.".into(),
    })
}

fn media_inline(input: &Value) -> Result<WorkflowPlan, String> {
    let Some(video_path) = input
        .get("video_path")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(WorkflowPlan::Degraded {
            message: "Choose a local video before creating the inline GIF.".into(),
        });
    };
    if !Path::new(video_path).is_file() {
        return Ok(WorkflowPlan::Degraded {
            message: "That video is no longer available on this Mac.".into(),
        });
    }
    Ok(WorkflowPlan::Job {
        kind: "video-to-gif".into(),
        input: json!({
            "input": video_path,
            "start": input.get("start").and_then(Value::as_f64).unwrap_or(0.0),
            "duration": input.get("duration").and_then(Value::as_f64).unwrap_or(8.0),
            "fps": 12,
            "max_dim": 720,
            "note_path": input.get("note_path").and_then(Value::as_str).unwrap_or("")
        }),
        message: "The capped GIF is running locally and will attach when complete.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeService;
    use std::fs;

    #[test]
    fn every_workflow_degrades_cleanly_without_required_input() {
        let directory = tempfile::tempdir().expect("temp");
        let database = Database::open_memory().expect("db");
        let notes = NotesService::new(directory.path().join("notes")).expect("notes");
        for definition in DEFINITIONS {
            if definition.id == "workflow-documentation" {
                continue;
            }
            let result = plan(&database, &notes, definition.id, &json!({})).expect("plan");
            assert!(
                matches!(result, WorkflowPlan::Degraded { .. }),
                "{} did not degrade",
                definition.id
            );
        }
        assert!(matches!(
            plan(&database, &notes, "workflow-documentation", &json!({})).expect("plan"),
            WorkflowPlan::Degraded { .. }
        ));
    }

    #[test]
    fn named_workflows_produce_jobs_notes_and_context() {
        let directory = tempfile::tempdir().expect("temp");
        let database = Database::open_memory().expect("db");
        let notes = NotesService::new(directory.path().join("notes")).expect("notes");
        let project = TimeService::projects(&database)
            .expect("projects")
            .into_iter()
            .next()
            .expect("project");
        TimeService::add_manual(&database, Some(project.id), "Delivery", 1_000, 4_600, true)
            .expect("entry");
        let note = notes
            .create(&database, "Project brief", "project-brief")
            .expect("note");
        notes
            .link_project(&database, &note.path, Some(project.id))
            .expect("link");

        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-billable-document",
                &json!({ "template": "invoice" })
            )
            .expect("billable"),
            WorkflowPlan::Job { .. }
        ));
        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-documentation",
                &json!({ "note_path": note.path })
            )
            .expect("documentation"),
            WorkflowPlan::Job { .. }
        ));
        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-research",
                &json!({ "source_name": "Paper.pdf", "source_text": "Local excerpt" })
            )
            .expect("research"),
            WorkflowPlan::Done { .. }
        ));
        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-context-switch",
                &json!({ "project_id": project.id })
            )
            .expect("context"),
            WorkflowPlan::Done { .. }
        ));
        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-standup",
                &json!({ "bullet": "Shipped the local workflow engine" })
            )
            .expect("standup"),
            WorkflowPlan::Done { .. }
        ));
        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-capture-attach",
                &json!({ "note_path": note.path })
            )
            .expect("capture"),
            WorkflowPlan::NextTool { .. }
        ));
        let clip = directory.path().join("clip.mp4");
        fs::write(&clip, b"fixture").expect("clip");
        assert!(matches!(
            plan(
                &database,
                &notes,
                "workflow-media-inline",
                &json!({ "video_path": clip })
            )
            .expect("media"),
            WorkflowPlan::Job { .. }
        ));
    }

    #[test]
    fn defaults_are_saved_idempotently() {
        let database = Database::open_memory().expect("db");
        install_defaults(&database).expect("install");
        install_defaults(&database).expect("reinstall");
        let count = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM workflows", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|error| error.to_string())
            })
            .expect("count");
        assert_eq!(count, DEFINITIONS.len() as i64);
    }
}
