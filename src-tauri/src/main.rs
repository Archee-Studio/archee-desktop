//! Archee Studio for the desktop: one native window around the web app. It adds no product
//! logic: the page decides everything, the shell only hosts it, marks the document as a desktop
//! host, lets the headers drag the window and sends foreign links to the browser.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;

/// Where the window points: `ARCHEE_URL` at build or run time, else the hosted app.
const DEFAULT_URL: &str = "https://app.archee.studio/chat";

/// Runs before the page's own scripts on every load.
const HOST_SCRIPT: &str = include_str!("host.js");

fn start_url() -> Url {
    let raw = std::env::var("ARCHEE_URL")
        .ok()
        .or_else(|| option_env!("ARCHEE_URL").map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_URL.to_owned());
    Url::parse(&raw).unwrap_or_else(|_| Url::parse(DEFAULT_URL).expect("default URL parses"))
}

/// The page may navigate within Archee and the local preview; anything else opens outside.
fn stays_inside(url: &Url, home: &Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    url.origin() == home.origin()
        || host == "archee.studio"
        || host.ends_with(".archee.studio")
        || host == "127.0.0.1"
        || host == "localhost"
        || matches!(url.scheme(), "tauri" | "about" | "data" | "blob")
}

fn main() {
    let home = start_url();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let inside = home.clone();
            let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(home.clone()))
                .title("Archee Studio")
                .inner_size(1440.0, 900.0)
                .min_inner_size(960.0, 600.0)
                // Windows (WebView2): this is the only switch for Ctrl +/-/0 and pinch, both off by
                // default. The page's pinch guard cancels the pinch; the keys keep working.
                // macOS (WKWebView) ignores it: host.js maps the keys to set_zoom there, and a
                // pinch reaches the page as gesture events the same guard cancels.
                .zoom_hotkeys_enabled(true)
                .initialization_script(HOST_SCRIPT)
                .on_navigation(move |url| {
                    if stays_inside(url, &inside) {
                        return true;
                    }
                    let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
                    false
                });
            #[cfg(target_os = "macos")]
            let builder = builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true);
            builder.build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Archee Studio failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archee_and_the_local_preview_stay_inside() {
        let home = Url::parse(DEFAULT_URL).unwrap();
        for inside in [
            "https://app.archee.studio/chat/abc",
            "https://dev.archee.studio/login",
            "https://docs.archee.studio/",
            "http://127.0.0.1:3010/chat",
        ] {
            assert!(stays_inside(&Url::parse(inside).unwrap(), &home), "{inside}");
        }
    }

    #[test]
    fn foreign_links_open_in_the_browser() {
        let home = Url::parse(DEFAULT_URL).unwrap();
        for outside in ["https://github.com/x", "https://archee.studio.evil.com/", "mailto:a@b.c"] {
            assert!(!stays_inside(&Url::parse(outside).unwrap(), &home), "{outside}");
        }
    }
}
