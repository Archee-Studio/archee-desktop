//! Where the window may go. The page navigates within Archee and, in dev builds, the local
//! preview; every other link opens in the system browser. A page may open its own generated
//! ``blob:`` or ``data:`` document in a window of the app, but neither scheme may take over the
//! app's own window: a link could swap the app for a page it wrote, dressed as the real thing.

use url::Url;

/// What the load is for. The app's own window navigates nowhere a page's document could pose as
/// Archee; a window the page opened may hold its generated ``blob:`` and ``data:`` documents.
#[derive(Clone, Copy)]
pub(crate) enum Load {
    Navigate,
    Open,
}

/// The hook also sees frames, so the sign-in check's Cloudflare Turnstile frame must stay in:
/// sent to the browser, it never loads and the check never finishes.
fn stays_inside(url: &Url, home: &Url, load: Load) -> bool {
    let open = matches!(load, Load::Open);
    // A blob URL belongs to the page that made it, which `blob:<origin>/<id>` names; blobs can
    // nest one scheme per layer, so unwrap a bounded few rather than recurse without limit.
    let mut url = url;
    let mut inner;
    for _ in 0..8 {
        if url.scheme() != "blob" {
            break;
        }
        if !open {
            return false;
        }
        let Ok(parsed) = Url::parse(url.path()) else {
            return false;
        };
        inner = parsed;
        url = &inner;
    }
    if url.scheme() == "blob" {
        return false;
    }
    let host = url.host_str().unwrap_or_default();
    // Host checks name web pages only: without them `file://archee.studio/x` or `ssh://` would
    // ride the domain match into the window.
    let web = matches!(url.scheme(), "http" | "https");
    url.origin() == home.origin()
        || (url.scheme() == "https" && host == "challenges.cloudflare.com")
        || (web && host == "archee.studio")
        || (web && host.ends_with(".archee.studio"))
        // The local preview is a dev-build surface; a shipped window sends it to the browser,
        // where it cannot pose as the app to a less technical user. A `tauri build --debug`
        // bundle still accepts it.
        || (cfg!(debug_assertions) && web && matches!(host, "127.0.0.1" | "localhost"))
        || matches!(url.scheme(), "tauri" | "about")
        || (open && url.scheme() == "data")
}

/// Whether the app's own window may navigate to `url` (a link, a redirect, `location=`).
pub(crate) fn navigates_inside(url: &Url, home: &Url) -> bool {
    stays_inside(url, home, Load::Navigate)
}

/// Whether a window the page opens (`target=_blank`, `window.open`) may hold `url`: the same set
/// as navigation, plus the generated ``blob:`` and ``data:`` documents the page previews.
pub(crate) fn opens_inside(url: &Url, home: &Url) -> bool {
    stays_inside(url, home, Load::Open)
}

/// Whether a page-opened `url` cannot live in an app window and leaves for the browser instead.
pub(crate) fn opens_elsewhere(url: &Url, home: &Url) -> bool {
    !opens_inside(url, home)
}

/// Whether a link may be handed to the system: web and mail links only. The page is remote, so it
/// never reaches a `file:` path or another application's URL scheme.
fn may_leave(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https" | "mailto")
}

/// Hands a link to the system browser or mail client, when `may_leave` allows it.
pub(crate) fn open_outside(url: &Url) {
    if may_leave(url) {
        // A link that fails to open leaves the page where it is; there is no one to tell.
        drop(open::that_detached(url.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "https://app.archee.studio/chat";

    #[test]
    fn archee_and_the_local_preview_stay_inside() {
        let home = Url::parse(HOME).unwrap();
        for inside in [
            "https://app.archee.studio/chat/abc",
            "https://dev.archee.studio/login",
            "https://docs.archee.studio/",
            "http://127.0.0.1:3010/chat",
            "https://challenges.cloudflare.com/cdn-cgi/challenge-platform/h/g/turnstile/if/ov2/av0/rcv/x",
            "about:blank",
        ] {
            let url = Url::parse(inside).unwrap();
            assert!(
                navigates_inside(&url, &home) && opens_inside(&url, &home),
                "{inside}"
            );
            assert!(!opens_elsewhere(&url, &home), "{inside}");
        }
    }

    #[test]
    fn generated_documents_open_but_never_take_over_the_window() {
        let home = Url::parse(HOME).unwrap();
        for generated in [
            "blob:https://app.archee.studio/6f1c2a3e-0d5b-4c1e-9a77-1f2b3c4d5e6f",
            "data:text/plain;base64,aGk=",
        ] {
            let url = Url::parse(generated).unwrap();
            assert!(opens_inside(&url, &home), "{generated}");
            assert!(
                !navigates_inside(&url, &home),
                "{generated} must not replace the app"
            );
        }
    }

    #[test]
    fn a_domain_match_still_needs_a_web_scheme() {
        let home = Url::parse(HOME).unwrap();
        for sneaky in [
            "file://archee.studio/etc/passwd",
            "ssh://git.archee.studio/repo",
            "wss://app.archee.studio/",
            "file://127.0.0.1/etc/passwd",
            "file://localhost/etc/passwd",
        ] {
            let url = Url::parse(sneaky).unwrap();
            assert!(
                !stays_inside(&url, &home, Load::Navigate)
                    && !stays_inside(&url, &home, Load::Open),
                "{sneaky}"
            );
        }
    }

    #[test]
    fn blob_documents_unwrap_within_a_bound() {
        let home = Url::parse(HOME).unwrap();
        let nested = Url::parse("blob:blob:https://app.archee.studio/id").unwrap();
        assert!(stays_inside(&nested, &home, Load::Open));
        assert!(!stays_inside(&nested, &home, Load::Navigate));
        let malformed = Url::parse("blob:not-a-url").unwrap();
        assert!(!stays_inside(&malformed, &home, Load::Open));
        let mut deep = "https://app.archee.studio/x".to_owned();
        for _ in 0..10 {
            deep = format!("blob:{deep}");
        }
        let deep = Url::parse(&deep).unwrap();
        assert!(!stays_inside(&deep, &home, Load::Open));
    }

    #[test]
    fn foreign_links_open_in_the_browser() {
        let home = Url::parse(HOME).unwrap();
        for outside in [
            "https://github.com/x",
            "https://archee.studio.evil.com/",
            "mailto:a@b.c",
            "https://www.cloudflare.com/",
            "http://challenges.cloudflare.com/",
            "blob:https://evil.example/6f1c2a3e",
        ] {
            let url = Url::parse(outside).unwrap();
            assert!(
                !navigates_inside(&url, &home) && !opens_inside(&url, &home),
                "{outside}"
            );
            assert!(opens_elsewhere(&url, &home), "{outside}");
        }
    }

    #[test]
    fn only_web_and_mail_links_may_leave() {
        for yes in [
            "https://github.com/x",
            "http://example.com/",
            "mailto:a@b.c",
        ] {
            assert!(may_leave(&Url::parse(yes).unwrap()), "{yes}");
        }
        for no in [
            "file:///etc/passwd",
            "ssh://host/",
            "zoommtg://x",
            "tel:123",
        ] {
            assert!(!may_leave(&Url::parse(no).unwrap()), "{no}");
        }
    }
}
