//! The page-side script the shell injects. The page's own CSS reads `html[data-host="desktop"]`
//! and `data-os`, and Tauri's drag script reads `data-tauri-drag-region`; this maps the page's
//! `data-drag-region` and `data-no-drag` onto it, so the web keeps one vocabulary.

/// The script with `__OS__` still to fill in.
const TEMPLATE: &str = r#"(() => {
  const mark = (root) => {
    if (root.dataset.host !== "desktop") root.dataset.host = "desktop";
    if (root.dataset.os !== "__OS__") root.dataset.os = "__OS__";
  };
  const watch = (root) => {
    mark(root);
    new MutationObserver(() => mark(root)).observe(root, {
      attributes: true,
      attributeFilter: ["data-host", "data-os"],
    });
  };
  if (document.documentElement) watch(document.documentElement);
  else
    new MutationObserver((_, seen) => {
      if (!document.documentElement) return;
      seen.disconnect();
      watch(document.documentElement);
    }).observe(document, { childList: true });

  // Tauri's drag script reads data-tauri-drag-region on mousedown. The capture phase runs before
  // it, so the attribute is set from the page's own marks just in time and never during hydration.
  addEventListener(
    "mousedown",
    (event) => {
      for (const el of event.composedPath()) {
        if (!(el instanceof Element)) continue;
        if (el.hasAttribute("data-no-drag")) el.setAttribute("data-tauri-drag-region", "false");
        else if (el.hasAttribute("data-drag-region")) el.setAttribute("data-tauri-drag-region", "deep");
      }
    },
    true,
  );
})();"#;

/// The value the page's CSS sees in `data-os`: `macos` or `other`.
const OS: &str = if cfg!(target_os = "macos") {
    "macos"
} else {
    "other"
};

/// The initialization script for every window. It runs before the page's own scripts, before the
/// document has its root element, and hydration may rewrite the root's attributes, so the marks
/// are applied once the root exists and restored if the page drops them.
pub(crate) fn script() -> String {
    TEMPLATE.replace("__OS__", OS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_names_the_os_and_leaves_no_placeholder() {
        let script = script();
        assert!(!script.contains("__OS__"), "{script}");
        assert!(script.contains(&format!("\"{OS}\"")), "{script}");
        assert!(
            script.contains(r#"root.dataset.host = "desktop""#),
            "{script}"
        );
    }

    #[test]
    fn the_script_maps_the_pages_drag_marks_onto_tauris() {
        let script = script();
        for needle in [
            r#"hasAttribute("data-no-drag")"#,
            r#"setAttribute("data-tauri-drag-region", "false")"#,
            r#"hasAttribute("data-drag-region")"#,
            r#"setAttribute("data-tauri-drag-region", "deep")"#,
        ] {
            assert!(script.contains(needle), "{needle}");
        }
    }
}
