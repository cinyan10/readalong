mod cefr;
mod commands;
mod db;
mod dictionary;
mod epub;
mod models;

use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use rusqlite::Connection;
#[cfg(target_os = "macos")]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

pub struct AppState {
    data_dir: PathBuf,
    db: Mutex<Connection>,
}

pub struct ExitState {
    approved: AtomicBool,
}

#[tauri::command]
fn complete_app_exit(app: tauri::AppHandle, exit_state: tauri::State<'_, ExitState>) {
    exit_state.approved.store(true, Ordering::SeqCst);
    app.exit(0);
}

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            let connection = db::connect(&data_dir.join("readalong.sqlite3"))?;
            app.manage(AppState {
                data_dir,
                db: Mutex::new(connection),
            });
            app.manage(ExitState {
                approved: AtomicBool::new(false),
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("app-leave-requested", ());
            }
        });

    #[cfg(target_os = "macos")]
    let builder = builder
        .menu(|app| {
            let quit =
                MenuItem::with_id(app, "request-quit", "Quit Readalong", true, Some("Cmd+Q"))?;
            let app_menu = Submenu::with_items(
                app,
                "Readalong",
                true,
                &[
                    &PredefinedMenuItem::about(app, None, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?;
            let file_menu = Submenu::with_items(
                app,
                "File",
                true,
                &[&PredefinedMenuItem::close_window(app, None)?],
            )?;
            let edit_menu = Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?;
            let view_menu = Submenu::with_items(
                app,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app, None)?],
            )?;
            let window_menu = Submenu::with_items(
                app,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, None)?,
                    &PredefinedMenuItem::maximize(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::close_window(app, None)?,
                ],
            )?;
            let help_menu = Submenu::with_items(app, "Help", true, &[])?;
            Menu::with_items(
                app,
                &[
                    &app_menu,
                    &file_menu,
                    &edit_menu,
                    &view_menu,
                    &window_menu,
                    &help_menu,
                ],
            )
        })
        .on_menu_event(|app, event| {
            if event.id() == "request-quit" {
                let _ = app.emit("app-leave-requested", ());
            }
        });

    let app = builder
        .invoke_handler(tauri::generate_handler![
            commands::list_books,
            commands::import_books,
            commands::get_reader,
            commands::get_chapter,
            commands::search_book,
            commands::lookup_word,
            commands::list_wordlist_entries,
            commands::list_book_wordlist_entries,
            commands::add_wordlist_entry,
            commands::delete_wordlist_entry,
            commands::list_book_highlights,
            commands::toggle_highlight,
            commands::get_part_audio,
            commands::get_part_alignment,
            commands::generate_part_audio,
            commands::sync_part_alignment,
            commands::save_progress,
            commands::save_bookmark,
            complete_app_exit,
        ])
        .build(tauri::generate_context!())
        .expect("error while running Readalong");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let exit_state = app.state::<ExitState>();
            if exit_state.approved.swap(false, Ordering::SeqCst) {
                return;
            }
            api.prevent_exit();
            let _ = app.emit("app-leave-requested", ());
        }
    });
}
