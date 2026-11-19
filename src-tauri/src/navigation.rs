//! Where the window may go. The page navigates within Archee and the local preview; every other
//! link opens in the system browser.

use url::Url;

/// The hook also sees frames, so the sign-in check's Cloudflare Turnstile frame must stay in:
/// sent to the browser, it never loads and the check never finishes.
pub(crate) fn stays_inside(url: &Url, home: &Url) -> bool {
    // A blob URL belongs to the page that made it, which `blob:<origin>/<id>` names.
    if url.scheme() == "blob" {
        return Url::parse(url.path()).is_ok_and(|origin| stays_inside(&origin, home));
    }
    let host = url.host_str().unwrap_or_default();
    url.origin() == home.origin()
        || (url.scheme() == "https" && host == "challenges.cloudflare.com")
        || host == "archee.studio"
        || host.ends_with(".archee.studio")
        || host == "127.0.0.1"
        || host == "localhost"
        || matches!(url.scheme(), "tauri" | "about" | "data")
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
            "blob:https://app.archee.studio/6f1c2a3e-0d5b-4c1e-9a77-1f2b3c4d5e6f",
            "data:text/plain;base64,aGk=",
            "about:blank",
        ] {
            assert!(
                stays_inside(&Url::parse(inside).unwrap(), &home),
                "{inside}"
            );
        }
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
            assert!(!stays_inside(&url, &home), "{outside}");
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
