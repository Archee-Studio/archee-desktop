const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

const source = fs.readFileSync(new URL("./src/host.js", `file://${__filename}`), "utf8");

function boot({ saved = null, readError = null, writeError = null } = {}) {
  const listeners = new Map();
  const zoom = [];
  const fail = (name) => {
    throw Object.assign(new Error("storage failure"), { name });
  };
  const context = {
    navigator: { platform: "MacIntel" },
    document: {
      documentElement: { dataset: {} },
      addEventListener() {},
    },
    MutationObserver: class {
      observe() {}
    },
    Element: class {},
    localStorage: {
      getItem() {
        if (readError) fail(readError);
        return saved;
      },
      setItem(_key, value) {
        if (writeError) fail(writeError);
        saved = value;
      },
    },
    window: {
      __TAURI__: {
        webview: { getCurrentWebview: () => ({ setZoom: (level) => zoom.push(level) }) },
      },
    },
    addEventListener(name, listener) {
      listeners.set(name, listener);
    },
  };
  vm.runInNewContext(source, context, { timeout: 1000 });
  listeners.get("DOMContentLoaded")?.();
  return {
    zoom,
    press(key) {
      assert.equal(typeof listeners.get("keydown"), "function");
      let prevented = false;
      listeners.get("keydown")({
        key,
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        preventDefault: () => (prevented = true),
      });
      assert.equal(prevented, true);
    },
  };
}

for (const name of ["SecurityError", "QuotaExceededError"]) {
  const denied = boot({ readError: name, writeError: name });
  for (const key of ["=", "=", "-", "0"]) denied.press(key);
  assert.deepEqual(denied.zoom, [1.1, 1.25, 1.1, 1]);
}

const restored = boot({ saved: "1.25" });
restored.press("=");
assert.deepEqual(restored.zoom, [1.25, 1.5]);
const invalid = boot({ saved: "99" });
invalid.press("=");
assert.deepEqual(invalid.zoom, [1.1]);
assert.throws(() => boot({ readError: "TypeError" }), { name: "TypeError" });
const unexpected = boot({ writeError: "TypeError" });
assert.throws(() => unexpected.press("="), { name: "TypeError" });
const bounded = boot({ readError: "SecurityError", writeError: "SecurityError" });
for (let i = 0; i < 30; i++) bounded.press("=");
assert.equal(bounded.zoom.at(-1), 3);
for (let i = 0; i < 40; i++) bounded.press("-");
assert.equal(bounded.zoom.at(-1), 0.5);
