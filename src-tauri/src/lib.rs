mod db;
mod registry;

use std::fs;

use db::{Database, DbStatus};
use registry::{Registry, ToolCtx, ToolInput, ToolMeta, ToolOutcome};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, PhysicalPosition, Position, Runtime, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(registry::build_registry())
        .manage(ToolCtx)
        .setup(|app| {
            let data_root = app.path().data_dir()?.join("Tooldeck");
            fs::create_dir_all(&data_root)?;
            let database = Database::open(data_root.join("tooldeck.sqlite3"))?;
            app.manage(database);

            let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            let handler_shortcut = shortcut;
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app_handle, received, event| {
                        if received == &handler_shortcut && event.state() == ShortcutState::Pressed
                        {
                            reveal_palette(app_handle);
                        }
                    })
                    .build(),
            )?;
            if let Err(error) = app.global_shortcut().register(shortcut) {
                eprintln!("Tooldeck could not register ⌥Space: {error}");
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
        .invoke_handler(tauri::generate_handler![tool_catalog, tool_run, db_status])
        .run(tauri::generate_context!())
        .expect("error while running Tooldeck");
}
