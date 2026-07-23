mod db;
mod notes;
mod platform;
mod registry;
mod time;

use std::fs;

use db::{Database, DbStatus};
use notes::{NoteDocument, NoteSummary, NotesService};
use registry::{Registry, ToolCtx, ToolInput, ToolMeta, ToolOutcome};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, PhysicalPosition, Position, Runtime, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use time::{IdleDecision, Project, TimeEntry, TimeService, TimerReport};

#[tauri::command]
fn tool_catalog(registry: tauri::State<'_, Registry>) -> Vec<ToolMeta> {
    registry.catalog()
}

#[tauri::command]
fn tool_run(
    registry: tauri::State<'_, Registry>,
    context: tauri::State<'_, ToolCtx>,
    id: String,
    input: ToolInput,
) -> Result<ToolOutcome, String> {
    registry
        .get(&id)
        .ok_or_else(|| format!("unknown tool: {id}"))?
        .run(context.inner(), input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn db_status(database: tauri::State<'_, Database>) -> Result<DbStatus, String> {
    database.status()
}

#[tauri::command]
fn notes_list(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
) -> Result<Vec<NoteSummary>, String> {
    notes.list(&database)
}

#[tauri::command]
fn note_get(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
    path: String,
) -> Result<NoteDocument, String> {
    notes.get(&database, &path)
}

#[tauri::command]
fn note_create(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
    title: String,
    template: String,
) -> Result<NoteDocument, String> {
    notes.create(&database, &title, &template)
}

#[tauri::command]
fn note_save(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
    path: String,
    content: String,
    expected_updated_at: i64,
) -> Result<NoteDocument, String> {
    notes.save(&database, &path, &content, expected_updated_at)
}

#[tauri::command]
fn note_daily(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
) -> Result<NoteDocument, String> {
    notes.daily(&database)
}

#[tauri::command]
fn notes_search(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
    query: String,
) -> Result<Vec<NoteSummary>, String> {
    notes.search(&database, &query)
}

#[tauri::command]
fn note_backlinks(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
    path: String,
) -> Result<Vec<NoteSummary>, String> {
    notes.backlinks(&database, &path)
}

#[tauri::command]
fn note_link_project(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
    path: String,
    project_id: Option<i64>,
) -> Result<(), String> {
    notes.link_project(&database, &path, project_id)
}

#[tauri::command]
fn floating_note_get(
    notes: tauri::State<'_, NotesService>,
    database: tauri::State<'_, Database>,
) -> Result<NoteDocument, String> {
    notes.floating_get(&database)
}

#[tauri::command]
fn projects_list(database: tauri::State<'_, Database>) -> Result<Vec<Project>, String> {
    TimeService::projects(&database)
}

#[tauri::command]
fn project_create(
    database: tauri::State<'_, Database>,
    name: String,
    color: String,
    billable: bool,
    rate: f64,
) -> Result<Project, String> {
    TimeService::create_project(&database, &name, &color, billable, rate)
}

#[tauri::command]
fn timer_status(database: tauri::State<'_, Database>) -> Result<Option<TimeEntry>, String> {
    TimeService::status(&database)
}

#[tauri::command]
fn timer_start(
    database: tauri::State<'_, Database>,
    project_id: Option<i64>,
    note_path: Option<String>,
    description: String,
    billable: bool,
) -> Result<TimeEntry, String> {
    TimeService::start(
        &database,
        project_id,
        note_path.as_deref(),
        &description,
        billable,
    )
}

#[tauri::command]
fn timer_stop(database: tauri::State<'_, Database>) -> Result<TimeEntry, String> {
    TimeService::stop(&database)
}

#[tauri::command]
fn timer_report(
    database: tauri::State<'_, Database>,
    day_start: i64,
    day_end: i64,
) -> Result<TimerReport, String> {
    TimeService::report(&database, day_start, day_end)
}

#[tauri::command]
fn timer_add_manual(
    database: tauri::State<'_, Database>,
    project_id: Option<i64>,
    description: String,
    started_at: i64,
    stopped_at: i64,
    billable: bool,
) -> Result<TimeEntry, String> {
    TimeService::add_manual(
        &database,
        project_id,
        &description,
        started_at,
        stopped_at,
        billable,
    )
}

#[tauri::command]
fn timer_edit(
    database: tauri::State<'_, Database>,
    id: i64,
    project_id: Option<i64>,
    description: String,
    started_at: i64,
    stopped_at: i64,
    billable: bool,
) -> Result<TimeEntry, String> {
    TimeService::edit_entry(
        &database,
        id,
        project_id,
        &description,
        started_at,
        stopped_at,
        billable,
    )
}

#[tauri::command]
fn timer_split(
    database: tauri::State<'_, Database>,
    id: i64,
    split_at: i64,
) -> Result<Vec<TimeEntry>, String> {
    TimeService::split_entry(&database, id, split_at)
}

#[tauri::command]
fn timer_merge(
    database: tauri::State<'_, Database>,
    first_id: i64,
    second_id: i64,
) -> Result<TimeEntry, String> {
    TimeService::merge_entries(&database, first_id, second_id)
}

#[tauri::command]
fn timer_reconcile_idle(
    database: tauri::State<'_, Database>,
    idle_seconds: i64,
    decision: IdleDecision,
    target_project_id: Option<i64>,
) -> Result<Option<TimeEntry>, String> {
    TimeService::reconcile_idle(&database, idle_seconds, decision, target_project_id)
}

#[tauri::command]
fn show_floating_note(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("floating-note")
        .ok_or_else(|| "floating note window is unavailable".to_string())?;
    window.show().map_err(|error| error.to_string())
}

#[tauri::command]
fn idle_seconds() -> f64 {
    platform::idle_seconds()
}

fn tray_image() -> tauri::image::Image<'static> {
    const WIDTH: u32 = 18;
    const HEIGHT: u32 = 18;
    let mut rgba = vec![0_u8; (WIDTH * HEIGHT * 4) as usize];
    for layer in 0..3_u32 {
        let offset_x = 3 + layer * 2;
        let offset_y = 2 + layer * 2;
        for y in offset_y..(offset_y + 9) {
            for x in offset_x..(offset_x + 8) {
                let border =
                    x == offset_x || x == offset_x + 7 || y == offset_y || y == offset_y + 8;
                if border {
                    let index = ((y * WIDTH + x) * 4) as usize;
                    rgba[index..index + 4].copy_from_slice(&[0, 0, 0, 210]);
                }
            }
        }
    }
    tauri::image::Image::new_owned(rgba, WIDTH, HEIGHT)
}

fn toggle_menu_bar<R: Runtime>(window: &WebviewWindow<R>, position: PhysicalPosition<f64>) {
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        let _ = window.hide();
        return;
    }

    if let Ok(scale) = window.scale_factor() {
        let width = 310.0 * scale;
        let x = (position.x - width + 18.0).max(0.0);
        let y = position.y + (23.0 * scale);
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(
            x.round() as i32,
            y.round() as i32,
        )));
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn reveal_palette<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("tooldeck://open-palette", ());
    }
}

fn toggle_floating_note<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("floating-note") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(registry::build_registry())
        .manage(ToolCtx)
        .setup(|app| {
            let data_root = app.path().data_dir()?.join("Tooldeck");
            fs::create_dir_all(&data_root)?;
            let database = Database::open(data_root.join("tooldeck.sqlite3"))?;
            let notes = NotesService::new(data_root.join("notes"))?;
            notes.reindex(&database)?;
            app.manage(database);
            app.manage(notes);

            let palette_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            let note_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyN);
            let handler_palette_shortcut = palette_shortcut;
            let handler_note_shortcut = note_shortcut;
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app_handle, received, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        if received == &handler_palette_shortcut {
                            reveal_palette(app_handle);
                        } else if received == &handler_note_shortcut {
                            toggle_floating_note(app_handle);
                        }
                    })
                    .build(),
            )?;
            if let Err(error) = app.global_shortcut().register(palette_shortcut) {
                eprintln!("Tooldeck could not register ⌥Space: {error}");
            }
            if let Err(error) = app.global_shortcut().register(note_shortcut) {
                eprintln!("Tooldeck could not register ⌥N: {error}");
            }

            TrayIconBuilder::with_id("tooldeck")
                .icon(tray_image())
                .icon_as_template(true)
                .tooltip("Tooldeck · No timer")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        position,
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("menu-bar") {
                            toggle_menu_bar(&window, position);
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tool_catalog,
            tool_run,
            db_status,
            notes_list,
            note_get,
            note_create,
            note_save,
            note_daily,
            notes_search,
            note_backlinks,
            note_link_project,
            floating_note_get,
            projects_list,
            project_create,
            timer_status,
            timer_start,
            timer_stop,
            timer_report,
            timer_add_manual,
            timer_edit,
            timer_split,
            timer_merge,
            timer_reconcile_idle,
            idle_seconds,
            show_floating_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tooldeck");
}
