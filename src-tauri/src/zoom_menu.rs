//! The macOS side of zoom. WKWebView has no zoom keys of its own, so the app menu carries Zoom In,
//! Zoom Out and Actual Size with the usual shortcuts and sets the page zoom of every window.

use std::sync::{Mutex, PoisonError};

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::zoom::{SAVED, SHORTCUTS, Zoom, command_for};

/// The managed state: the level every window shares.
#[derive(Debug)]
struct Level(Mutex<Zoom>);

/// The macOS menu: the standard one, with the zoom commands at the top of View.
///
/// # Errors
///
/// Returns the error of the first menu item the system refuses to create.
pub(crate) fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let name = app.package_info().name.clone();
    let mut zoom = Vec::new();
    for shortcut in SHORTCUTS {
        zoom.push(MenuItem::with_id(
            app,
            shortcut.id,
            shortcut.label,
            true,
            Some(shortcut.accelerator),
        )?);
    }
    let view = Submenu::new(app, "View", true)?;
    for item in &zoom {
        view.append(item)?;
    }
    view.append(&PredefinedMenuItem::separator(app)?)?;
    view.append(&PredefinedMenuItem::fullscreen(app, None)?)?;
    Menu::with_items(
        app,
        &[
            &Submenu::with_items(
                app,
                name,
                true,
                &[
                    &PredefinedMenuItem::about(app, None, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "File",
                true,
                &[&PredefinedMenuItem::close_window(app, None)?],
            )?,
            &Submenu::with_items(
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
            )?,
            &view,
            &Submenu::with_items(
                app,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, None)?,
                    &PredefinedMenuItem::maximize(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::close_window(app, None)?,
                ],
            )?,
        ],
    )
}

/// Reads the saved level and keeps it as the app's state. Call once, before any window exists.
pub(crate) fn init(app: &AppHandle) {
    let saved = app.path().app_config_dir().ok().map(|dir| dir.join(SAVED));
    app.manage(Level(Mutex::new(Zoom::load(saved))));
}

/// Applies the current level to a new window; Actual Size is already what it shows.
pub(crate) fn sync(window: &WebviewWindow) {
    let level = window.state::<Level>();
    let zoom = level.0.lock().unwrap_or_else(PoisonError::into_inner);
    if !zoom.is_actual() {
        // The page stays at Actual Size when this fails; there is no one to tell.
        drop(window.set_zoom(zoom.factor()));
    }
}

/// Handles a click or shortcut on a zoom command and zooms every window; other menu events are
/// not ours.
pub(crate) fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let Some(command) = command_for(event.id().as_ref()) else {
        return;
    };
    let level = app.state::<Level>();
    let mut zoom = level.0.lock().unwrap_or_else(PoisonError::into_inner);
    // Best effort: an unwritable config folder only costs the level at the next start.
    drop(zoom.step(command));
    for window in app.webview_windows().values() {
        // Same as in `sync`: a failed zoom leaves the page as it is.
        drop(window.set_zoom(zoom.factor()));
    }
}
