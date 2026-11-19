//! The windows: the main one on the web app, and the in-app windows the page opens with
//! `target=_blank` or `window.open`. Both get the same host script and link policy.

use std::sync::atomic::{AtomicU32, Ordering};

use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::Url;

use crate::{host, navigation};

/// The label of the main window; the window-state plugin and the single-instance hook name it.
pub(crate) const MAIN: &str = "main";

/// Labels of windows the page opened start with this; `capabilities/default.json` lists the glob.
const OPENED_PREFIX: &str = "opened-";

/// Makes each opened window's label unique within a run.
static OPENED: AtomicU32 = AtomicU32::new(0);

/// Whether the window-state plugin should remember this window: the main one only.
pub(crate) fn remembered(label: &str) -> bool {
    label == MAIN
}

#[expect(clippy::expect_used, reason = "the literal parses")]
fn blank() -> Url {
    Url::parse("about:blank").expect("about:blank parses")
}

/// The builder both kinds of window share. It must run on the main thread or inside a handler.
fn host_window<'a, M: Manager<tauri::Wry>>(
    manager: &'a M,
    label: &str,
    url: WebviewUrl,
    home: &Url,
) -> WebviewWindowBuilder<'a, tauri::Wry, M> {
    let app = manager.app_handle().clone();
    let (inside, nested) = (home.clone(), home.clone());
    let builder = WebviewWindowBuilder::new(manager, label, url)
        .title("Archee Studio")
        .inner_size(1440.0, 900.0)
        .min_inner_size(960.0, 600.0)
        // Windows (WebView2): this is the only switch for Ctrl +/-/0 and pinch, both off by
        // default. The page's pinch guard cancels the pinch; the keys keep working.
        // macOS (WKWebView) ignores it: the View menu (`zoom_menu`) carries the keys there, and
        // a pinch reaches the page as gesture events the same guard cancels.
        .zoom_hotkeys_enabled(true)
        .initialization_script(host::script())
        .on_navigation(move |url| {
            if navigation::stays_inside(url, &inside) {
                return true;
            }
            navigation::open_outside(url);
            false
        })
        .on_new_window(move |url, features| open_new(&app, &nested, &url, features));
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    builder
}

/// Opens the main window on `home`.
///
/// # Errors
///
/// Returns the error of the system refusing to create the window or its webview.
pub(crate) fn open_main(app: &AppHandle, home: &Url) -> tauri::Result<WebviewWindow> {
    host_window(app, MAIN, WebviewUrl::External(home.clone()), home).build()
}

/// The page asked for a new window. Archee's own pages, `blob:` and `data:` documents open in a
/// window of the app, so the session and the opener come with them; anything else, on the web or
/// by mail, goes to the browser.
fn open_new(
    app: &AppHandle,
    home: &Url,
    url: &Url,
    features: NewWindowFeatures,
) -> NewWindowResponse<tauri::Wry> {
    if !navigation::stays_inside(url, home) {
        navigation::open_outside(url);
        return NewWindowResponse::Deny;
    }
    let label = format!("{OPENED_PREFIX}{}", OPENED.fetch_add(1, Ordering::Relaxed));
    // The system loads `url` into the webview it is given, so the builder starts on a blank page.
    let built = host_window(app, &label, WebviewUrl::External(blank()), home)
        .window_features(features)
        .build();
    match built {
        Ok(window) => {
            #[cfg(target_os = "macos")]
            crate::zoom_menu::sync(&window);
            NewWindowResponse::Create { window }
        }
        // There is no one to tell; the page sees a blocked pop-up.
        Err(_) => NewWindowResponse::Deny,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_main_window_is_remembered() {
        assert!(remembered(MAIN));
        assert!(!remembered(&format!("{OPENED_PREFIX}3")));
    }

    #[test]
    fn opened_windows_match_the_capability_glob() {
        let capability = include_str!("../capabilities/default.json");
        assert!(
            capability.contains(&format!("\"{OPENED_PREFIX}*\"")),
            "{capability}"
        );
    }
}
