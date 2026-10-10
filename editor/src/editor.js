import { autocompletion } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import {
  bracketMatching,
  defaultHighlightStyle,
  foldGutter,
  HighlightStyle,
  foldKeymap,
  indentOnInput,
  StreamLanguage,
  syntaxHighlighting,
} from "@codemirror/language";
import { linter, setDiagnostics } from "@codemirror/lint";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { EditorState } from "@codemirror/state";
import { tags } from "@lezer/highlight";
import {
  crosshairCursor,
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";

const root = document.documentElement;
const token = location.hash.slice(1);
history.replaceState(null, "", location.pathname);
const $ = (id) => document.getElementById(id);
const status = $("status-message");
const tabs = new Map();
let activePath = null;
let editor = null;
let diagnostics = [];
let treeFiles = [];
let activeFilter = "";
const editorRoot = $("editor-root");
const messages = {
  de: {
    subtitle: "Lokale Entwicklungsumgebung", local: "LOKAL", files: "DATEIEN", newFile: "Neue Datei",
    refresh: "Aktualisieren", filterFiles: "Dateien filtern", saveAll: "Alle speichern", save: "Speichern",
    switchLanguage: "Sprache wechseln",
    format: "Formatieren", check: "Projekt prüfen", welcome: "Willkommen in Zelyra Studio",
    welcomeHint: "Öffne links eine Datei und entwickle direkt in deinem Projekt.", problems: "Probleme",
    noProblems: "Keine Probleme gefunden. Dein Projekt ist bereit.", close: "Panel schließen", loadingFiles: "Dateien werden geladen",
    toggleExplorer: "Dateiübersicht umschalten", search: "Suchen", handbook: "Handbuch", projectFiles: "Projektdateien",
    openFile: "Datei öffnen", complete: "Vervollständigen", ready: "Bereit", localMode: "Lokaler Modus",
    noDocument: "Kein Dokument geöffnet", opened: "{0} geöffnet", unsaved: "Ungespeicherte Änderungen",
    saved: "{0} gespeichert", checkRunning: "Projekt wird geprüft …", checkPassed: "Projektprüfung erfolgreich",
    problemsFound: "{0} Problem(e) gefunden", formatRunning: "Formatiere …", formatted: "{0} formatiert",
    createPrompt: "Neuer Dateipfad im Projekt (zum Beispiel src/customer.zyl)",
    closeUnsaved: "{0} enthält ungespeicherte Änderungen. Tab trotzdem schließen?",
    filesCount: "{0} Dateien", chooseFile: "Datei öffnen"
  },
  en: {
    subtitle: "Local development environment", local: "LOCAL", files: "FILES", newFile: "New file",
    refresh: "Refresh", filterFiles: "Filter files", saveAll: "Save all", save: "Save",
    switchLanguage: "Switch language",
    format: "Format", check: "Check project", welcome: "Welcome to Zelyra Studio",
    welcomeHint: "Open a file on the left and work directly in your project.", problems: "Problems",
    noProblems: "No problems found. Your project is ready.", close: "Close panel", loadingFiles: "Loading files",
    toggleExplorer: "Toggle file explorer", search: "Search", handbook: "Handbook", projectFiles: "Project files",
    openFile: "Open file", complete: "Complete", ready: "Ready", localMode: "Local mode",
    noDocument: "No document open", opened: "Opened {0}", unsaved: "Unsaved changes",
    saved: "Saved {0}", checkRunning: "Checking project …", checkPassed: "Project check passed",
    problemsFound: "Found {0} problem(s)", formatRunning: "Formatting …", formatted: "Formatted {0}",
    createPrompt: "New project file path (for example src/customer.zyl)",
    closeUnsaved: "{0} has unsaved changes. Close this tab anyway?",
    filesCount: "{0} files", chooseFile: "Open file"
  }
};
let language = localStorage.getItem("zelyra-editor-language") === "en" ? "en" : "de";
function t(key, value) { return (messages[language][key] || key).replace("{0}", value ?? ""); }
function setLanguage(next) {
  language = next;
  document.documentElement.lang = language;
  $("language-button").textContent = language.toUpperCase();
  for (const element of document.querySelectorAll("[data-i18n]")) element.textContent = messages[language][element.dataset.i18n] || element.textContent;
  for (const element of document.querySelectorAll("[data-i18n-title]")) element.title = messages[language][element.dataset.i18nTitle] || element.title;
  for (const element of document.querySelectorAll("[data-i18n-placeholder]")) element.placeholder = messages[language][element.dataset.i18nPlaceholder] || element.placeholder;
  for (const element of document.querySelectorAll("[data-i18n-aria]")) element.setAttribute("aria-label", messages[language][element.dataset.i18nAria] || element.getAttribute("aria-label"));
  localStorage.setItem("zelyra-editor-language", language);
  renderTree();
}

const keywords = new Set((
  "fn let mut if else match for in while loop break continue return type record " +
  "table view component layout slot page api form crud tableview auth database " +
  "sql uses capability requires ensures invariant true false null none some ok err"
).split(/\s+/));
const zelyraLanguage = StreamLanguage.define({
  startState: () => ({ inBlockComment: false }),
  token(stream, state) {
    if (state.inBlockComment) {
      if (stream.match("*/")) state.inBlockComment = false;
      else stream.next();
      return "comment";
    }
    if (stream.eatSpace()) return null;
    if (stream.match("//")) { stream.skipToEnd(); return "comment"; }
    if (stream.match("/*")) {
      state.inBlockComment = true;
      if (stream.skipTo("*/")) { stream.match("*/"); state.inBlockComment = false; }
      return "comment";
    }
    if (stream.match(/(?:\"(?:[^\"\\]|\\.)*\"|'(?:[^'\\]|\\.)*')/)) return "string";
    if (stream.match(/\b\d+(?:\.\d+)?\b/)) return "number";
    if (stream.match(/[A-Za-z_][\w]*/)) {
      const word = stream.current();
      if (keywords.has(word)) return "keyword";
      if (/^[A-Z]/.test(word)) return "typeName";
      return "variableName";
    }
    if (stream.match(/[{}()[\],.;:]/)) return "punctuation";
    if (stream.match(/[=<>!+*/%&|?-]+/)) return "operator";
    stream.next();
    return null;
  },
});

const zelyraTheme = EditorView.theme({
  "&": { height: "100%", fontSize: "14px", backgroundColor: "var(--editor-bg)", color: "var(--ink)" },
  ".cm-scroller": { fontFamily: "'JetBrains Mono', 'Cascadia Code', ui-monospace, monospace", lineHeight: "1.65", overflow: "auto" },
  ".cm-content": { padding: "18px 0", caretColor: "var(--accent)" },
  ".cm-gutters": { backgroundColor: "var(--editor-gutter)", color: "var(--muted)", border: "none", minWidth: "52px" },
  ".cm-activeLineGutter": { color: "var(--ink)" },
  ".cm-activeLine": { backgroundColor: "var(--editor-active)" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { background: "var(--selection)" },
  ".cm-tooltip": { border: "1px solid var(--border)", backgroundColor: "var(--surface)", color: "var(--ink)", boxShadow: "0 12px 35px #11182722" },
  ".cm-tooltip-autocomplete ul li[aria-selected]": { background: "var(--accent)", color: "white" },
  ".cm-lintRange-error": { backgroundImage: "none", borderBottom: "2px wavy var(--danger)" },
}, { dark: document.documentElement.dataset.theme === "dark" });
const zelyraHighlight = syntaxHighlighting(HighlightStyle.define([
  { tag: tags.keyword, color: "var(--token-keyword)", fontWeight: "600" },
  { tag: tags.typeName, color: "var(--token-type)" },
  { tag: tags.variableName, color: "var(--token-name)" },
  { tag: tags.string, color: "var(--token-string)" },
  { tag: tags.number, color: "var(--token-number)" },
  { tag: tags.comment, color: "var(--token-comment)", fontStyle: "italic" },
  { tag: tags.operator, color: "var(--token-operator)" },
  { tag: tags.punctuation, color: "var(--token-punctuation)" },
]));

async function api(path, options = {}) {
  const headers = new Headers(options.headers || {});
  headers.set("X-Zelyra-Editor-Token", token);
  if (options.body && !headers.has("Content-Type")) headers.set("Content-Type", "application/json");
  const response = await fetch(`/__zelyra/editor/api/${path}`, { ...options, headers, cache: "no-store", credentials: "omit", mode: "same-origin", redirect: "error" });
  const body = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
  if (!response.ok) throw new Error(body.error || `HTTP ${response.status}`);
  return body;
}

function announce(message, kind = "") {
  status.textContent = message;
  status.dataset.kind = kind;
}

function isDirty(tab) { return tab && tab.state.doc.toString() !== tab.saved; }
function dirtyCount() { return [...tabs.values()].filter(isDirty).length; }

function renderTabs() {
  const bar = $("tabs");
  bar.replaceChildren();
  for (const [path, tab] of tabs) {
    const button = document.createElement("button");
    button.className = `tab${path === activePath ? " active" : ""}`;
    button.title = path;
    const dot = document.createElement("span");
    dot.className = `tab-state${isDirty(tab) ? " dirty" : ""}`;
    const label = document.createElement("span");
    label.textContent = path.split("/").pop();
    button.append(dot, label);
    button.addEventListener("click", () => activate(path));
    const close = document.createElement("span");
    close.className = "tab-close";
    close.textContent = "×";
    close.title = "Tab schließen";
    close.addEventListener("click", (event) => { event.stopPropagation(); closeTab(path); });
    button.append(close);
    bar.append(button);
  }
  $("save-button").disabled = !activePath || !isDirty(tabs.get(activePath));
  $("save-all-button").disabled = dirtyCount() === 0;
}

function completionSource(context) {
  const word = context.matchBefore(/[\w]+/);
  if (!word && !context.explicit) return null;
  return { from: word ? word.from : context.pos, options: [...keywords].map((label) => ({ label, type: "keyword" })) };
}

function createView(path, state) {
  return new EditorView({
    state,
    parent: editorRoot,
    extensions: [
      lineNumbers(), highlightActiveLineGutter(), foldGutter(), history(), drawSelection(),
      rectangularSelection(), crosshairCursor(), highlightActiveLine(), bracketMatching(),
      indentOnInput(), highlightSelectionMatches(), zelyraLanguage,
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }), zelyraHighlight,
      autocompletion({ override: [completionSource], activateOnTyping: true }),
      linter(() => diagnostics.filter((item) => !item.file || item.file.endsWith(path)).map((item) => ({
        from: Math.max(0, Math.min(state.doc.length, item.from)),
        to: Math.max(0, Math.min(state.doc.length, item.to)),
        severity: item.severity || "error",
        message: `${item.code ? `${item.code}: ` : ""}${item.message}`,
      }))),
      keymap.of([
        { key: "Mod-s", run: () => { saveActive(); return true; } },
        ...defaultKeymap, ...historyKeymap, ...searchKeymap, ...foldKeymap,
      ]),
      EditorView.updateListener.of((update) => {
        const tab = tabs.get(path);
        if (!tab) return;
        tab.state = update.state;
        if (update.docChanged) { setDiagnostics(update.view, []); renderTabs(); announce(t("unsaved"), "warning"); }
      }),
      EditorView.lineWrapping,
      zelyraTheme,
      EditorView.contentAttributes.of({ "aria-label": `${path} im Zelyra-Webeditor` }),
    ],
  });
}

async function openFile(path) {
  if (tabs.has(path)) return activate(path);
  try {
    const file = await api(`file?path=${encodeURIComponent(path)}`);
    const tab = { saved: file.content, hash: file.hash, state: EditorState.create({ doc: file.content }) };
    tabs.set(path, tab);
    activate(path);
    announce(t("opened", path));
  } catch (error) { announce(error.message, "error"); }
}

function activate(path) {
  if (!tabs.has(path)) return;
  if (editor) { tabs.get(activePath).state = editor.state; editor.destroy(); }
  activePath = path;
  const tab = tabs.get(path);
  editor = createView(path, tab.state);
  $("active-file").textContent = path;
  $("language-label").textContent = path.endsWith(".zyl") ? "Zelyra" : path.split(".").pop().toUpperCase();
  $("empty-state").hidden = true;
  renderTabs();
  editor.focus();
}

function closeTab(path) {
  const tab = tabs.get(path);
  if (isDirty(tab) && !confirm(t("closeUnsaved", path))) return;
  if (activePath === path && editor) { editor.destroy(); editor = null; }
  tabs.delete(path);
  if (activePath === path) {
    activePath = null;
    const next = tabs.keys().next().value;
    if (next) activate(next);
    else { $("active-file").textContent = t("noDocument"); $("empty-state").hidden = false; }
  }
  renderTabs();
}

async function savePath(path) {
  const tab = tabs.get(path);
  if (!tab || !isDirty(tab)) return true;
  try {
    const result = await api("file", { method: "PUT", body: JSON.stringify({ path, content: tab.state.doc.toString(), baseHash: tab.hash }) });
    tab.saved = result.content;
    tab.hash = result.hash;
    renderTabs();
    announce(t("saved", path), "success");
    return true;
  } catch (error) { announce(error.message, "error"); return false; }
}
function saveActive() { if (activePath) void savePath(activePath); }
async function saveAll() {
  for (const path of tabs.keys()) if (!(await savePath(path))) return false;
  return true;
}

function makeTree(files) {
  const rootNode = { children: new Map(), path: "" };
  for (const path of files) {
    let node = rootNode;
    const parts = path.split("/");
    parts.forEach((part, index) => {
      if (!node.children.has(part)) node.children.set(part, { path: parts.slice(0, index + 1).join("/"), children: new Map(), file: index === parts.length - 1 });
      node = node.children.get(part);
    });
  }
  return rootNode;
}

function renderTree() {
  const container = $("file-tree");
  container.replaceChildren();
  const tree = makeTree(treeFiles.filter((path) => path.toLowerCase().includes(activeFilter.toLowerCase())));
  const draw = (node, target, depth = 0) => {
    [...node.children.entries()].sort((a, b) => Number(a[1].file) - Number(b[1].file) || a[0].localeCompare(b[0])).forEach(([name, item]) => {
      const row = document.createElement("button");
      row.className = `tree-item${item.file ? " file" : " folder"}${item.path === activePath ? " selected" : ""}`;
      row.style.setProperty("--depth", depth);
      row.title = item.path;
      const icon = document.createElement("span"); icon.className = "tree-icon"; icon.textContent = item.file ? fileIcon(name) : "▸";
      const label = document.createElement("span"); label.className = "tree-label"; label.textContent = name;
      row.append(icon, label);
      row.addEventListener("click", () => item.file ? openFile(item.path) : row.classList.toggle("collapsed"));
      target.append(row);
      if (!item.file) {
        const children = document.createElement("div"); children.className = "tree-children";
        row.after(children); draw(item, children, depth + 1);
        row.addEventListener("click", () => { children.hidden = row.classList.contains("collapsed"); });
      }
    });
  };
  draw(tree, container);
  $("file-count").textContent = t("filesCount", treeFiles.length);
}
function fileIcon(name) { return name.endsWith(".zyl") ? "λ" : name.endsWith(".md") ? "M" : name.endsWith(".css") ? "#" : "·"; }

async function refreshTree() {
  try { treeFiles = (await api("tree")).files; renderTree(); }
  catch (error) { announce(error.message, "error"); }
}

function renderDiagnostics(items, success) {
  diagnostics = items.map((item) => {
    const start = item.span?.start || {};
    return { ...item, from: item.editorSpan?.start ?? start.offset ?? 0, to: item.editorSpan?.end ?? item.editorSpan?.start ?? start.offset ?? 0 };
  });
  if (editor) setDiagnostics(editor, diagnostics.filter((item) => !item.file || item.file.endsWith(activePath)).map((item) => ({
    from: item.from, to: Math.max(item.from + 1, item.to), severity: item.severity || "error",
    message: `${item.code || ""} ${item.message || ""}`.trim(),
  })));
  const panel = $("problems-list"); panel.replaceChildren();
  for (const item of items) {
    const row = document.createElement("button"); row.className = `problem ${item.severity || "error"}`;
    const title = document.createElement("strong"); title.textContent = `${item.code || "Fehler"} · ${item.file || ""}`;
    const message = document.createElement("span"); message.textContent = item.message || "Unbekannte Diagnose";
    const pos = item.span?.start;
    const location = document.createElement("small"); location.textContent = pos ? `Zeile ${pos.line}, Spalte ${pos.column}` : "";
    row.append(title, message, location);
    row.addEventListener("click", () => {
      const file = item.file;
      if (file) void openFile(file).then(() => {
        if (activePath === file && editor && pos) editor.dispatch({ selection: { anchor: item.from }, scrollIntoView: true });
      });
    });
    panel.append(row);
  }
  $("problems-count").textContent = String(items.length);
  $("problems-count-status").textContent = String(items.length);
  $("problems-panel").hidden = false;
  $("problems-empty").hidden = items.length > 0;
  announce(success ? t("checkPassed") : t("problemsFound", items.length), success ? "success" : "error");
}

async function checkProject() {
  if (!(await saveAll())) return;
  announce(t("checkRunning"));
  try {
    const result = await api("check", { method: "POST", body: JSON.stringify({ path: activePath }) });
    renderDiagnostics(result.diagnostics || [], result.success);
  } catch (error) { announce(error.message, "error"); }
}

async function formatActive() {
  if (!activePath) return;
  if (!(await savePath(activePath))) return;
  announce(t("formatRunning"));
  try {
    const result = await api("format", { method: "POST", body: JSON.stringify({ path: activePath, baseHash: tabs.get(activePath).hash }) });
    const tab = tabs.get(activePath);
    tab.saved = result.content; tab.hash = result.hash;
    tab.state = EditorState.create({ doc: result.content });
    if (editor) { editor.setState(tab.state); tab.state = editor.state; }
    renderTabs();
    announce(t("formatted", activePath), "success");
  } catch (error) { announce(error.message, "error"); }
}

async function createFile() {
  const path = prompt(t("createPrompt"));
  if (!path) return;
  try {
    await api("file", { method: "POST", body: JSON.stringify({ path, content: "" }) });
    await refreshTree(); await openFile(path);
  } catch (error) { announce(error.message, "error"); }
}

$("save-button").addEventListener("click", saveActive);
$("save-all-button").addEventListener("click", saveAll);
$("check-button").addEventListener("click", checkProject);
$("format-button").addEventListener("click", formatActive);
$("new-file-button").addEventListener("click", createFile);
$("refresh-button").addEventListener("click", refreshTree);
$("tree-filter").addEventListener("input", (event) => { activeFilter = event.target.value; renderTree(); });
$("theme-button").addEventListener("click", () => {
  root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
  localStorage.setItem("zelyra-editor-theme", root.dataset.theme);
});
$("language-button").addEventListener("click", () => setLanguage(language === "de" ? "en" : "de"));
$("sidebar-toggle").addEventListener("click", () => document.body.classList.toggle("sidebar-hidden"));
$("problems-toggle").addEventListener("click", () => { $("problems-panel").hidden = !$("problems-panel").hidden; });
$("problems-close").addEventListener("click", () => { $("problems-panel").hidden = true; });
$("open-search").addEventListener("click", () => { if (editor) { editor.focus(); import("@codemirror/search").then(({ openSearchPanel }) => openSearchPanel(editor)); } });
window.addEventListener("beforeunload", (event) => {
  if (dirtyCount()) { event.preventDefault(); event.returnValue = ""; }
});
window.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); saveActive(); }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "p") {
    event.preventDefault();
    const query = prompt("Datei öffnen");
    if (query) { const match = treeFiles.find((path) => path.toLowerCase().includes(query.toLowerCase())); if (match) void openFile(match); }
  }
});

if (["light", "dark"].includes(localStorage.getItem("zelyra-editor-theme"))) root.dataset.theme = localStorage.getItem("zelyra-editor-theme");
setLanguage(language);
$("project-name").textContent = document.body.dataset.project || "Zelyra-Projekt";
$("project-name-side").textContent = (document.body.dataset.project || "ZELYRA-PROJEKT").toLocaleUpperCase();
refreshTree().then(() => {
  const first = treeFiles.includes("main.zyl") ? "main.zyl" : treeFiles.find((file) => file.endsWith(".zyl"));
  if (first) void openFile(first);
});
