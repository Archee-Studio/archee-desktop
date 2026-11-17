// Marks the page as hosted by the desktop shell and wires what CSS app-region cannot do in a
// WebKit webview: headers marked data-drag-region drag the window, a double click zooms it, and
// controls inside them stay clickable. The page's own CSS reads html[data-host="desktop"].
// This runs before the document has its root element, and hydration may rewrite the root's
// attributes, so the marks are applied once the root exists and restored if they are dropped.
(() => {
  const os = /Mac/.test(navigator.platform) ? "macos" : "other";
  const mark = (root) => {
    if (root.dataset.host !== "desktop") root.dataset.host = "desktop";
    if (root.dataset.os !== os) root.dataset.os = os;
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

  const CONTROL = "button, a, input, textarea, select, [role='button'], [data-no-drag]";
  addEventListener("mousedown", (event) => {
    if (event.button !== 0 || !(event.target instanceof Element)) return;
    if (!event.target.closest("[data-drag-region]") || event.target.closest(CONTROL)) return;
    const win = window.__TAURI__?.window?.getCurrentWindow?.();
    if (!win) return;
    event.preventDefault();
    if (event.detail === 2) win.toggleMaximize();
    else win.startDragging();
  });

  // WKWebView has no zoom keys of its own (WebView2 gets them from the window builder), so map
  // ⌘+ ⌘− ⌘0 to the webview's page zoom. The level survives reloads in localStorage.
  if (os === "macos") {
    const KEY = "archee.desktop.zoom";
    const STEPS = [0.5, 0.67, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];
    const readSaved = () => {
      try {
        return localStorage.getItem(KEY);
      } catch (error) {
        if (error?.name === "SecurityError" || error?.name === "QuotaExceededError") return null;
        throw error;
      }
    };
    const apply = (level) => {
      current = level;
      try {
        localStorage.setItem(KEY, String(level));
      } catch (error) {
        if (error?.name !== "SecurityError" && error?.name !== "QuotaExceededError") throw error;
      }
      window.__TAURI__?.webview?.getCurrentWebview?.().setZoom(level);
    };
    const saved = Number(readSaved());
    let current = STEPS.includes(saved) ? saved : 1;
    if (current !== 1) addEventListener("DOMContentLoaded", () => apply(current));
    addEventListener("keydown", (event) => {
      if (!event.metaKey || event.ctrlKey || event.altKey) return;
      const at = STEPS.indexOf(current);
      let next = null;
      if (event.key === "=" || event.key === "+") next = STEPS[Math.min(STEPS.length - 1, at + 1)];
      else if (event.key === "-") next = STEPS[Math.max(0, at - 1)];
      else if (event.key === "0") next = 1;
      if (next === null) return;
      event.preventDefault();
      apply(next);
    });
  }
})();
