//! Archee Studio for the desktop: one native window around the web app. It adds no product
//! logic: the page decides everything, the shell only hosts it, marks the document as a desktop
//! host, lets the headers drag the window (Tauri's own `data-tauri-drag-region`) and sends
//! foreign links to the browser.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod host;
mod navigation;
mod window;
// Only the macOS menu drives zoom, but its logic is plain Rust and its tests run everywhere.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "used by the macOS menu only")
)]
mod zoom;
#[cfg(target_os = "macos")]
mod zoom_menu;

use tauri::Manager;
use url::Url;

/// Where the window points: `ARCHEE_URL` at build or run time, else the hosted app.
const DEFAULT_URL: &str = "https://app.archee.studio/chat";

#[expect(clippy::expect_used, reason = "DEFAULT_URL is a literal that parses")]
fn start_url() -> Url {
    let raw = std::env::var("ARCHEE_URL")
        .ok()
        .or_else(|| option_env!("ARCHEE_URL").map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_URL.to_owned());
    Url::parse(&raw).unwrap_or_else(|_| Url::parse(DEFAULT_URL).expect("default URL parses"))
}

fn main() -> tauri::Result<()> {
    let home = start_url();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window(window::MAIN) {
                drop(window.unminimize()); // best effort: this callback has no caller to report to
                drop(window.set_focus());
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_filter(window::remembered)
                .build(),
        );
    #[cfg(target_os = "macos")]
    let builder = builder
        .menu(zoom_menu::menu)
        .on_menu_event(zoom_menu::on_menu_event);
    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            zoom_menu::init(app.handle());
            let window = window::open_main(app.handle(), &home)?;
            #[cfg(target_os = "macos")]
            zoom_menu::sync(&window);
            #[cfg(not(target_os = "macos"))]
            drop(window);
            Ok(())
        })
        .run(tauri::generate_context!())
}
