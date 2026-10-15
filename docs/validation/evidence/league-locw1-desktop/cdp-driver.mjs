// Minimal CDP driver (Node 24 built-in WebSocket) for the LOC-W1 desktop smoke. Sends real DOM
// events into the app's own webview (WebView2 remote debugging on :9333). Usage: node cdp.mjs <step>
import fs from "node:fs";
import path from "node:path";

const PORT = 9333;
const OUT = path.join(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1")), "shots");
fs.mkdirSync(OUT, { recursive: true });
const ROOT = String.raw`H:\TEMP\claude\e--Repositories-nhl-modding-studio\4eb07012-dc85-4291-bcf8-32adbae7e13e\scratchpad\desktop\root`;
const DICT = ROOT + String.raw`\dev_hdd0\game\BLUS31540\USRDIR\fe\loc\nhl_eng_us.db`;

const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
const page = targets.find((t) => t.type === "page" && t.url.includes("localhost:1420")) ?? targets.find((t) => t.type === "page");
if (!page) throw new Error("no page target: " + JSON.stringify(targets));
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
ws.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
};
function send(method, params = {}) {
  return new Promise((resolve) => {
    const i = ++id;
    pending.set(i, resolve);
    ws.send(JSON.stringify({ id: i, method, params }));
  });
}
async function evaluate(expr) {
  const r = await send("Runtime.evaluate", { expression: `(() => { ${HELPERS}; return eval(${JSON.stringify(expr)}); })()`, awaitPromise: true, returnByValue: true });
  if (r.result?.exceptionDetails) throw new Error(JSON.stringify(r.result.exceptionDetails));
  return r.result?.result?.value;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function shot(name) {
  const r = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(path.join(OUT, name + ".png"), Buffer.from(r.result.data, "base64"));
  console.log("shot", name);
}
const HELPERS = `window.__h || (window.__h = {
    setInput(el, value) {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
      setter.call(el, value);
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
    },
    byText(tag, text) {
      return Array.from(document.querySelectorAll(tag)).find((e) => e.textContent.trim() === text) || null;
    },
    byLabel(rx) {
      const r = new RegExp(rx);
      return Array.from(document.querySelectorAll("input,button")).find((e) => r.test(e.getAttribute("aria-label") || "")) || null;
    },
    byPlaceholder(rx) {
      const r = new RegExp(rx);
      return Array.from(document.querySelectorAll("input")).find((e) => r.test(e.placeholder || "")) || null;
    },
    text() { return document.body.innerText; },
  })`;
async function click(finder) {
  const ok = await evaluate(`(() => { const el = ${finder}; if (!el) return false; el.click(); return true; })()`);
  if (!ok) throw new Error("not found: " + finder);
}
async function setInput(finder, value) {
  const ok = await evaluate(`(() => { const el = ${finder}; if (!el) return false; window.__h.setInput(el, ${JSON.stringify(value)}); return true; })()`);
  if (!ok) throw new Error("input not found: " + finder);
}
async function waitText(rx, ms = 20000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    const t = await evaluate("window.__h.text()");
    if (new RegExp(rx).test(t)) return t;
    await sleep(250);
  }
  throw new Error("timeout waiting for " + rx);
}
const step = process.argv[2] || "all";
const log = (...a) => console.log(...a);

if (step === "all" || step === "open") {
  await evaluate(`location.hash = ""; location.href = location.origin + "/"; true`);
  await sleep(1500);
  await waitText("NHL Legacy");
  await shot("00-startup");
  await click(`Array.from(document.querySelectorAll("button")).find(b => /NHL Legacy/.test(b.textContent))`);
  await sleep(300);
  await click(`Array.from(document.querySelectorAll("button")).find(b => b.textContent.trim() === "PS3")`);
  await sleep(300);
  await setInput(`document.querySelector('input[placeholder]')`, ROOT);
  await click(`window.__h.byText("button", "Detect")`);
  await waitText("nhlng.db|Open", 30000);
  await shot("01-detected");
  await click(`Array.from(document.querySelectorAll("button")).find(b => /^Open project/.test(b.textContent.trim()) && !b.disabled)`);
  await waitText("Teams", 30000);
  await sleep(1000);
  await shot("02-opened");
}
if (step === "all" || step === "leagues") {
  await click(`window.__h.byText("a", "Leagues")`);
  await sleep(1500);
  await waitText("Leagues");
  await shot("03-leagues-no-dictionary");
  if (!/names via eng_us dictionary/.test(await evaluate("window.__h.text()"))) {
    await setInput(`window.__h.byPlaceholder("dictionary path")`, DICT);
    await click(`window.__h.byText("button", "attach")`);
  } else {
    log("dictionary already attached by discovery (install's own nhl_eng_us.db)");
  }
  await waitText("names via eng_us dictionary", 20000);
  await sleep(500);
  await shot("04-leagues-dictionary-attached");
  const t = await evaluate("window.__h.text()");
  log("catalog rows mention Norway:", /Norway/.test(t), "Rest Of World:", /Rest Of World/.test(t));
  await click(`Array.from(document.querySelectorAll("a")).find(a => (a.getAttribute("href") || "").endsWith("leagues/7"))`);
  await sleep(1500);
  await waitText("Rename via dictionary");
  await shot("05-league-7-detail");
  await click(`window.__h.byText("button", "Preview")`);
  await waitText("Rest Of World", 20000);
  await sleep(300);
  await shot("06-rename-preview");
  await click(`window.__h.byText("button", "Hold rename")`);
  await sleep(1500);
  await shot("07-rename-held");
  await click(`window.__h.byText("a", "Pending Edits")`);
  await sleep(1500);
  await waitText("Localization dictionary", 20000);
  await shot("08-pending-dictionary");
}
async function clickIf(finder) { await evaluate(`(() => { const el = ${finder}; if (el) el.click(); return true; })()`); }
if (step === "all" || step === "save") {
  await clickIf(`window.__h.byText("a", "Pending Edits")`);
  await sleep(1500);
  await waitText("Copied-target|copied target", 20000);
  await setInput(`window.__h.byPlaceholder("copied root path")`, ROOT);
  await sleep(800);
  await click(`window.__h.byText("button", "Review save")`);
  await waitText("prepared review|Apply to copied target", 60000);
  await sleep(800);
  await shot("09-review");
  await click(`window.__h.byText("button", "Apply to copied target")`);
  await waitText("committed", 60000);
  await sleep(1000);
  await shot("10-applied");
  await click(`window.__h.byText("a", "Leagues")`); await sleep(1200); await click(`Array.from(document.querySelectorAll("a")).find(a => (a.getAttribute("href") || "").endsWith("leagues/7"))`);
  await sleep(1500);
  await shot("11-league-7-after-apply");
}
if (step === "all" || step === "restore") {
  await clickIf(`Array.from(document.querySelectorAll("a,button")).find(a => /Pending Edits/.test(a.textContent))`);
  await sleep(2000);
  await click(`Array.from(document.querySelectorAll("button")).find(b => b.textContent.trim() === "Restore" && !b.disabled)`);
  await waitText("Restore of .*: restored|restored", 120000);
  await sleep(1000);
  await shot("12-restored");
  await click(`window.__h.byText("a", "Leagues")`); await sleep(1200); await click(`Array.from(document.querySelectorAll("a")).find(a => (a.getAttribute("href") || "").endsWith("leagues/7"))`);
  await sleep(1500);
  await shot("13-league-7-after-restore");
}
ws.close();
process.exit(0);
