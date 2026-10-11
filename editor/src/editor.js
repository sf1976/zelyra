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
let databaseMap = null;
let databaseTables = [];
let selectedDatabaseTable = null;
let databaseFilter = "";
let crudPreviewValid = false;
let lastCrudPreviewRequest = null;
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
    filesCount: "{0} Dateien", chooseFile: "Datei öffnen", database: "Datenbank",
    databaseStudio: "DATENBANK STUDIO", databaseTitle: "Datenbank verstehen",
    databaseIntro: "Leseübersicht für DATABASE_URL mit compilergeprüfter CRUD-Vorschau.", databaseNotLoaded: "Noch nicht verbunden",
    refreshSchema: "Schema laden", backToCode: "Zurück zum Code", filterTables: "Tabellen filtern",
    databaseChooseTable: "Wähle links eine Tabelle", databaseChooseHint: "Studio zeigt Spalten, Schlüssel, Beziehungen und Quellcode-Abweichungen.",
    loadSchema: "Datenbankschema laden", columnsAndRelations: "Spalten und Beziehungen",
    schemaCompareHint: "Deklarierte und live vorhandene Merkmale im Vergleich.", schemaLegend: "Quelle · Datenbank",
    relationshipMap: "Tabellenbeziehungen", relationshipMapHint: "Wähle eine Tabelle in der Karte, um ihre Felder zu öffnen.", relationshipCount: "{0} Beziehungen", schemaMapLabel: "Datenbankschema mit Tabellenbeziehungen",
    relationMatched: "Durchgezogen · Quelle und Live", relationSourceOnly: "Gestrichelt · nur Quelle", relationDatabaseOnly: "Gepunktet · nur Datenbank",
    guidedBuilder: "GEFÜHRTER ASSISTENT", crudBuilderTitle: "CRUD für diese Tabelle erstellen",
    crudBuilderHint: "Felder auswählen, Zelyra-Code prüfen und erst danach ausdrücklich übernehmen.",
    sourceFile: "Quelldatei", resourceName: "Ressourcenname", displayTitle: "Anzeigetitel", sourceColumn: "QUELLE", databaseColumn: "DATENBANK", saveBeforePreview: "Speichere die Quelldatei vor der Vorschau.", noTargetFiles: "Keine importierte Zelyra-Quelldatei gefunden",
    visibleFields: "Sichtbare Felder", chooseFieldsHint: "Mindestens ein Feld auswählen",
    previewCrud: "Code prüfen und Vorschau zeigen", applyCrud: "Änderung übernehmen",
    generatedSource: "ERZEUGTER ZELYRA-CODE", sourceDiff: "QUELLTEXTÄNDERUNG", changeImpact: "AUSWIRKUNGEN",
    schemaLoading: "Lade Datenbankschema …", schemaLoaded: "Schema geladen · nur lesend",
    schemaLoadFailed: "Datenbankschema konnte nicht geladen werden", tablesCount: "{0} Tabellen",
    matched: "Live-Tabelle vorhanden", schemaDrift: "Schemaabweichung", declaredMissingLive: "In Datenbank nicht gefunden", liveUnmapped: "Nur live vorhanden",
    noTables: "Keine Tabellen gefunden", noTableMatch: "Keine passende Tabelle gefunden",
    noColumns: "Keine Spalten verfügbar", primaryKey: "Primärschlüssel", nullable: "NULL erlaubt",
    required: "Pflichtfeld", autoIncrement: "Automatische ID", unique: "Eindeutig",
    relation: "Beziehung", relationTo: "Verweist auf", declaredOnly: "Nur in Quelle",
    liveOnly: "Nur in Datenbank", typeMismatch: "Abweichender Typ", noRelationships: "Keine Beziehungen deklariert",
    tableNotDeclared: "Diese Live-Tabelle ist noch nicht in Zelyra deklariert. Lege zuerst eine geprüfte Tabellendeklaration an.",
    existingCrud: "Für diese Tabelle existiert bereits eine CRUD-Ressource.",
    previewRunning: "Prüfe erzeugten Code …", previewPassed: "Compilerprüfung bestanden · Änderung noch nicht übernommen",
    previewFailed: "Compilerprüfung fehlgeschlagen", previewRequired: "Erstelle zuerst eine gültige Vorschau.",
    applySuccess: "CRUD-Änderung übernommen", selectOneField: "Wähle mindestens ein Feld.",
    table: "Tabelle", field: "Feld", moduleUnknown: "Quellmodul unbekannt", selectTarget: "Quelldatei auswählen"
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
    filesCount: "{0} files", chooseFile: "Open file", database: "Database",
    databaseStudio: "DATABASE STUDIO", databaseTitle: "Understand your data",
    databaseIntro: "Read-only DATABASE_URL schema overview with a compiler-checked CRUD preview.", databaseNotLoaded: "Not connected",
    refreshSchema: "Load schema", backToCode: "Back to code", filterTables: "Filter tables",
    databaseChooseTable: "Choose a table on the left", databaseChooseHint: "Studio shows columns, keys, relations, and source drift.",
    loadSchema: "Load database schema", columnsAndRelations: "Columns and relations",
    schemaCompareHint: "Compare declared fields with the live database.", schemaLegend: "Source · database",
    relationshipMap: "Table relationships", relationshipMapHint: "Choose a table in the map to open its fields.", relationshipCount: "{0} relationships", schemaMapLabel: "Database schema and table relationships",
    relationMatched: "Solid · source and live", relationSourceOnly: "Dashed · source only", relationDatabaseOnly: "Dotted · database only",
    guidedBuilder: "GUIDED BUILDER", crudBuilderTitle: "Create CRUD for this table",
    crudBuilderHint: "Choose fields, check generated Zelyra code, then explicitly apply it.",
    sourceFile: "Source file", resourceName: "Resource name", displayTitle: "Display title", sourceColumn: "SOURCE", databaseColumn: "DATABASE", saveBeforePreview: "Save the source file before previewing.", noTargetFiles: "No imported Zelyra source file found",
    visibleFields: "Visible fields", chooseFieldsHint: "Choose at least one field",
    previewCrud: "Check code and preview", applyCrud: "Apply change",
    generatedSource: "GENERATED ZELYRA SOURCE", sourceDiff: "SOURCE CHANGE", changeImpact: "CHANGE IMPACT",
    schemaLoading: "Loading database schema …", schemaLoaded: "Schema loaded · read only",
    schemaLoadFailed: "Could not load database schema", tablesCount: "{0} tables",
    matched: "Table exists", schemaDrift: "Schema drift", declaredMissingLive: "Missing from database", liveUnmapped: "Live only",
    noTables: "No tables found", noTableMatch: "No matching table",
    noColumns: "No columns available", primaryKey: "Primary key", nullable: "Nullable",
    required: "Required", autoIncrement: "Auto increment", unique: "Unique",
    relation: "Relation", relationTo: "References", declaredOnly: "Source only",
    liveOnly: "Database only", typeMismatch: "Type differs", noRelationships: "No declared relations",
    tableNotDeclared: "This live table is not declared in Zelyra yet. Add a reviewed table declaration first.",
    existingCrud: "A CRUD resource already exists for this table.",
    previewRunning: "Checking generated source …", previewPassed: "Compiler check passed · change not yet applied",
    previewFailed: "Compiler check failed", previewRequired: "Create a valid preview first.",
    applySuccess: "CRUD change applied", selectOneField: "Choose at least one field.",
    table: "Table", field: "Field", moduleUnknown: "Source module unknown", selectTarget: "Choose source file"
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
  if (databaseMap) {
    renderDatabaseTables();
    if (selectedDatabaseTable) selectDatabaseTable(selectedDatabaseTable.name);
    else renderDatabaseGraph();
  }
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

function flattenDatabaseMap(map) {
  const declared = (map.modules || []).flatMap((module) => (module.tables || []).map((table) => ({ ...table, module: table.module || module.source_module })));
  const liveOnly = (map.unmapped_live_tables || []).map((table) => ({ ...table, status: "live_unmapped" }));
  return [...declared, ...liveOnly].sort((left, right) => String(left.name).localeCompare(String(right.name)));
}

async function loadDatabaseSchema() {
  const state = $("database-state");
  state.textContent = t("schemaLoading");
  state.dataset.kind = "pending";
  try {
    databaseMap = await api("database-schema");
    databaseTables = flattenDatabaseMap(databaseMap);
    state.textContent = `${t("schemaLoaded")} · ${databaseMap.backend || ""}`;
    state.dataset.kind = "success";
    renderDatabaseTables();
    renderDatabaseGraph();
    if (databaseTables.length) selectDatabaseTable(selectedDatabaseTable?.name || databaseTables[0].name);
    else showDatabaseWelcome(t("noTables"));
    announce(t("schemaLoaded"), "success");
  } catch (error) {
    databaseMap = null;
    databaseTables = [];
    state.textContent = t("schemaLoadFailed");
    state.dataset.kind = "error";
    renderDatabaseGraph();
    showDatabaseWelcome(error.message);
    renderDatabaseTables();
    announce(error.message, "error");
  }
}

function statusLabel(status) {
  return ({ matched: t("matched"), schema_drift: t("schemaDrift"), declared_missing_live: t("declaredMissingLive"), live_unmapped: t("liveUnmapped") })[status] || status;
}

function tableStatus(table) {
  if (table.status !== "matched") return table.status;
  const declared = new Map((table.declared_columns || []).map((column) => [column.name, column]));
  const live = new Map((table.live_columns || []).map((column) => [column.name, column]));
  if (declared.size !== live.size || [...declared].some(([name, column]) => !live.has(name) || columnSignature(column) !== columnSignature(live.get(name)))) return "schema_drift";
  if (relationSets(table).some((relation) => relation.state !== "matched")) return "schema_drift";
  return "matched";
}

function renderDatabaseTables() {
  const container = $("database-tables");
  if (!container) return;
  container.replaceChildren();
  const filtered = databaseTables.filter((table) => String(table.name).toLowerCase().includes(databaseFilter.toLowerCase()));
  $("database-count").textContent = t("tablesCount", filtered.length);
  if (!filtered.length) {
    const empty = document.createElement("p"); empty.className = "database-empty";
    empty.textContent = databaseTables.length ? t("noTableMatch") : t("noTables"); container.append(empty); return;
  }
  for (const table of filtered) {
    const row = document.createElement("button");
    row.className = `database-table-item${selectedDatabaseTable?.name === table.name ? " selected" : ""}`;
    row.type = "button";
    const status = tableStatus(table);
    const icon = document.createElement("span"); icon.className = "database-table-icon"; icon.textContent = status === "matched" ? "▤" : "◌";
    const copy = document.createElement("span"); copy.className = "database-table-copy";
    const name = document.createElement("strong"); name.textContent = table.name;
    const sub = document.createElement("small"); sub.textContent = `${statusLabel(status)} · ${(table.live_columns || table.declared_columns || []).length} ${t("field")}`;
    copy.append(name, sub); row.append(icon, copy);
    row.addEventListener("click", () => selectDatabaseTable(table.name));
    container.append(row);
  }
}

function renderDatabaseGraph() {
  const panel = $("database-graph-panel");
  const svg = $("database-graph");
  if (!panel || !svg) return;
  panel.hidden = !databaseTables.length;
  svg.replaceChildren();
  if (!databaseTables.length) return;

  const namespace = "http://www.w3.org/2000/svg";
  const nodeWidth = 190;
  const nodeHeight = 72;
  const columnGap = 225;
  const rowGap = 112;
  const columns = 3;
  const positions = new Map(databaseTables.map((table, index) => [table.name, {
    x: 16 + (index % columns) * columnGap,
    y: 35 + Math.floor(index / columns) * rowGap,
  }]));
  const width = 16 + (columns - 1) * columnGap + nodeWidth + 16;
  const height = 35 + Math.ceil(databaseTables.length / columns) * rowGap + 15;
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("width", width);
  svg.setAttribute("height", height);
  svg.setAttribute("aria-label", t("schemaMapLabel"));
  $("database-graph-count").textContent = t("relationshipCount", databaseTables.reduce((count, table) => count + relationSets(table).length, 0));

  const defs = document.createElementNS(namespace, "defs");
  const marker = document.createElementNS(namespace, "marker");
  marker.id = "database-arrow";
  marker.setAttribute("viewBox", "0 0 10 10");
  marker.setAttribute("refX", "9");
  marker.setAttribute("refY", "5");
  marker.setAttribute("markerWidth", "6");
  marker.setAttribute("markerHeight", "6");
  marker.setAttribute("orient", "auto-start-reverse");
  const arrow = document.createElementNS(namespace, "path");
  arrow.setAttribute("d", "M 0 0 L 10 5 L 0 10 z");
  arrow.setAttribute("class", "database-graph-arrow");
  marker.append(arrow);
  defs.append(marker);
  svg.append(defs);

  const edges = document.createElementNS(namespace, "g");
  edges.setAttribute("class", "database-graph-edges");
  for (const table of databaseTables) {
    const source = positions.get(table.name);
    for (const { relation, state } of relationSets(table)) {
      const target = positions.get(relation.references_table);
      if (!target) continue;
      const sx = source.x + nodeWidth;
      const sy = source.y + nodeHeight / 2;
      const tx = target.x;
      const ty = target.y + nodeHeight / 2;
      const curve = document.createElementNS(namespace, "path");
      const bend = Math.max(32, Math.abs(tx - sx) / 2);
      curve.setAttribute("d", `M ${sx} ${sy} C ${sx + bend} ${sy}, ${tx - bend} ${ty}, ${tx} ${ty}`);
      curve.setAttribute("class", `database-graph-edge${selectedDatabaseTable?.name === table.name || selectedDatabaseTable?.name === relation.references_table ? " related" : ""}`);
      curve.dataset.state = state;
      curve.setAttribute("marker-end", "url(#database-arrow)");
      curve.setAttribute("aria-label", `${table.name}.${relation.column} → ${relation.references_table}.${relation.references_column}`);
      edges.append(curve);
    }
  }
  svg.append(edges);

  const nodes = document.createElementNS(namespace, "g");
  nodes.setAttribute("class", "database-graph-nodes");
  for (const table of databaseTables) {
    const position = positions.get(table.name);
    const relations = relationSets(table);
    const group = document.createElementNS(namespace, "g");
    const status = tableStatus(table);
    group.setAttribute("class", `database-graph-node${selectedDatabaseTable?.name === table.name ? " selected" : ""}${status === "matched" ? "" : " drift"}`);
    group.setAttribute("transform", `translate(${position.x} ${position.y})`);
    group.setAttribute("tabindex", "0");
    group.setAttribute("role", "button");
    group.setAttribute("aria-label", `${table.name}, ${statusLabel(status)}`);
    const card = document.createElementNS(namespace, "rect");
    card.setAttribute("width", nodeWidth);
    card.setAttribute("height", nodeHeight);
    card.setAttribute("rx", "8");
    const title = document.createElementNS(namespace, "text");
    title.setAttribute("x", "12"); title.setAttribute("y", "27"); title.setAttribute("class", "database-graph-title");
    title.textContent = table.name.length > 24 ? `${table.name.slice(0, 21)}…` : table.name;
    const subtitle = document.createElementNS(namespace, "text");
    subtitle.setAttribute("x", "12"); subtitle.setAttribute("y", "49"); subtitle.setAttribute("class", "database-graph-subtitle");
    subtitle.textContent = `${statusLabel(status)} · ${(table.live_columns || table.declared_columns || []).length} ${t("field")} · ${relations.length} ↗`;
    group.append(card, title, subtitle);
    const open = () => { selectDatabaseTable(table.name); $("database-table-view").scrollIntoView({ behavior: "smooth", block: "start" }); };
    group.addEventListener("click", open);
    group.addEventListener("keydown", (event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); open(); } });
    nodes.append(group);
  }
  svg.append(nodes);
}

function showDatabaseWelcome(message = t("databaseChooseHint")) {
  $("database-welcome").hidden = false;
  $("database-table-view").hidden = true;
  $("database-welcome").querySelector("p").textContent = message;
  selectedDatabaseTable = null;
  renderDatabaseTables();
  renderDatabaseGraph();
}

function selectDatabaseTable(name) {
  selectedDatabaseTable = databaseTables.find((table) => table.name === name) || null;
  if (!selectedDatabaseTable) return showDatabaseWelcome();
  $("database-welcome").hidden = true;
  $("database-table-view").hidden = false;
  $("database-module").textContent = selectedDatabaseTable.module || t("moduleUnknown");
  $("database-table-name").textContent = selectedDatabaseTable.name;
  $("database-table-status").textContent = statusLabel(tableStatus(selectedDatabaseTable));
  const badge = $("database-status-badge");
  badge.textContent = statusLabel(tableStatus(selectedDatabaseTable));
  badge.dataset.state = tableStatus(selectedDatabaseTable);
  renderDatabaseColumns(selectedDatabaseTable);
  renderCrudBuilder(selectedDatabaseTable);
  renderDatabaseTables();
  renderDatabaseGraph();
}

function columnSignature(column) {
  return [column.type, column.nullable, column.primary_key, column.auto_increment, column.unique]
    .map((value) => String(value ?? "").toLowerCase()).join("|");
}

function columnDescription(column) {
  const flags = [];
  if (column.primary_key) flags.push("PK");
  if (column.auto_increment) flags.push("AUTO");
  if (column.unique) flags.push("UNIQUE");
  if (!column.nullable) flags.push("REQUIRED");
  return `${column.type || "?"}${column.nullable ? "?" : ""}${flags.length ? ` · ${flags.join(" · ")}` : ""}`;
}

function relationSets(table) {
  const source = table.foreign_keys || [];
  const live = table.live_foreign_keys || [];
  const key = (relation) => `${relation.column}|${relation.references_table}|${relation.references_column}`;
  const sourceByKey = new Map(source.map((relation) => [key(relation), relation]));
  const liveByKey = new Map(live.map((relation) => [key(relation), relation]));
  return [...new Set([...sourceByKey.keys(), ...liveByKey.keys()])].map((signature) => ({
    relation: sourceByKey.get(signature) || liveByKey.get(signature),
    state: sourceByKey.has(signature) && liveByKey.has(signature) ? "matched" : sourceByKey.has(signature) ? "source-only" : "database-only",
  }));
}

function renderDatabaseColumns(table) {
  const container = $("database-columns");
  container.replaceChildren();
  const declared = new Map((table.declared_columns || []).map((column) => [column.name, column]));
  const live = new Map((table.live_columns || []).map((column) => [column.name, column]));
  const names = [...new Set([...declared.keys(), ...live.keys()])].sort((a, b) => a.localeCompare(b));
  if (!names.length) {
    const empty = document.createElement("p"); empty.className = "database-empty"; empty.textContent = t("noColumns"); container.append(empty); return;
  }
  const heading = document.createElement("div"); heading.className = "database-column-row database-column-head";
  for (const label of [t("field"), t("sourceColumn"), t("databaseColumn")]) { const cell = document.createElement("span"); cell.textContent = label; heading.append(cell); }
  container.append(heading);
  for (const name of names) {
    const source = declared.get(name); const database = live.get(name);
    const row = document.createElement("div"); row.className = "database-column-row";
    const identity = document.createElement("div"); identity.className = "database-column-identity";
    const field = document.createElement("strong"); field.textContent = name;
    identity.append(field);
    const sourceType = document.createElement("code"); sourceType.textContent = source ? columnDescription(source) : t("declaredOnly");
    const dbType = document.createElement("code"); dbType.textContent = database ? columnDescription(database) : t("liveOnly");
    row.append(identity, sourceType, dbType);
    if (!source || !database) row.dataset.state = source ? "source-only" : "database-only";
    else if (columnSignature(source) !== columnSignature(database)) row.dataset.state = "changed";
    container.append(row);
  }
  const relations = relationSets(table);
  const relationPanel = document.createElement("div"); relationPanel.className = "database-relations";
  const relationTitle = document.createElement("strong"); relationTitle.textContent = t("relation"); relationPanel.append(relationTitle);
  if (!relations.length) { const empty = document.createElement("span"); empty.textContent = t("noRelationships"); relationPanel.append(empty); }
  for (const { relation, state } of relations) {
    const item = document.createElement("span"); item.className = "database-relation-chip"; item.dataset.state = state;
    item.textContent = `${relation.column}  →  ${relation.references_table}.${relation.references_column} · ${state === "matched" ? t("matched") : state === "source-only" ? t("declaredOnly") : t("liveOnly")}`; relationPanel.append(item);
  }
  container.append(relationPanel);
}

function renderCrudBuilder(table) {
  const builder = $("crud-builder");
  const resources = databaseMap?.crud_resources || [];
  const existing = resources.find((resource) => resource.table === table.name);
  const usable = table.status === "matched" && !existing;
  builder.hidden = !usable;
  $("crud-preview-grid").hidden = true;
  $("crud-diagnostics").hidden = true;
  $("crud-apply-button").disabled = true;
  crudPreviewValid = false;
  if (!usable) {
    const status = table.status === "matched" ? t("existingCrud") : t("tableNotDeclared");
    const hint = document.createElement("div"); hint.className = "database-builder-hint"; hint.textContent = status;
    const old = $("database-table-view").querySelector(".database-builder-hint"); old?.remove();
    $("database-table-view").append(hint); return;
  }
  $("database-table-view").querySelector(".database-builder-hint")?.remove();
  const targetSelect = $("crud-target-file"); targetSelect.replaceChildren();
  const sourceFiles = (databaseMap?.crud_targets || []).filter((file) => treeFiles.includes(file));
  if (!sourceFiles.length) {
    const option = document.createElement("option"); option.value = ""; option.textContent = t("noTargetFiles"); targetSelect.append(option);
  }
  for (const path of sourceFiles) { const option = document.createElement("option"); option.value = path; option.textContent = path; targetSelect.append(option); }
  if (sourceFiles.includes("main.zyl")) targetSelect.value = "main.zyl";
  targetSelect.disabled = sourceFiles.length === 0;
  const resource = table.name.split("_").filter(Boolean).map((part) => part[0]?.toUpperCase() + part.slice(1)).join("");
  $("crud-resource-name").value = resource;
  $("crud-title").value = resource;
  const fields = $("crud-fields"); fields.replaceChildren();
  const columns = table.declared_columns || [];
  const liveColumns = new Set((table.live_columns || []).map((column) => column.name));
  for (const column of columns) {
    const label = document.createElement("label"); label.className = "crud-field-option";
    const input = document.createElement("input"); input.type = "checkbox"; input.value = column.name;
    input.disabled = !liveColumns.has(column.name);
    input.checked = !input.disabled && !column.primary_key && !column.auto_increment;
    input.addEventListener("change", invalidateCrudPreview);
    const name = document.createElement("strong"); name.textContent = column.name;
    const type = document.createElement("small"); type.textContent = `${column.type}${column.nullable ? " · NULL" : ""}`;
    label.append(input, name, type); fields.append(label);
  }
  if (!fields.querySelector("input:checked") && fields.querySelector("input:not(:disabled)")) fields.querySelector("input:not(:disabled)").checked = true;
  $("crud-target-file").onchange = invalidateCrudPreview;
  $("crud-resource-name").oninput = invalidateCrudPreview;
  $("crud-title").oninput = invalidateCrudPreview;
}

function invalidateCrudPreview() {
  crudPreviewValid = false;
  lastCrudPreviewRequest = null;
  $("crud-apply-button").disabled = true;
  $("crud-preview-state").textContent = "";
}

function selectedCrudFields() {
  return [...$("crud-fields").querySelectorAll("input:checked")].map((input) => input.value);
}

async function currentCrudRequest() {
  const path = $("crud-target-file").value;
  if (!path) throw new Error(t("selectTarget"));
  if (!selectedCrudFields().length) throw new Error(t("selectOneField"));
  const tab = tabs.get(path);
  if (tab && isDirty(tab)) throw new Error(t("saveBeforePreview"));
  const file = await api(`file?path=${encodeURIComponent(path)}`);
  return {
    path,
    baseHash: file.hash,
    table: selectedDatabaseTable.name,
    fields: selectedCrudFields(),
    resourceName: $("crud-resource-name").value.trim(),
    title: $("crud-title").value.trim(),
  };
}

function showCrudDiagnostics(items) {
  const panel = $("crud-diagnostics"); panel.replaceChildren(); panel.hidden = !items.length;
  for (const item of items) {
    const row = document.createElement("div"); row.className = "crud-diagnostic";
    const code = document.createElement("strong"); code.textContent = item.code || "Compiler";
    const message = document.createElement("span"); message.textContent = item.message || "";
    row.append(code, message); panel.append(row);
  }
}

async function previewCrud() {
  invalidateCrudPreview();
  $("crud-preview-state").textContent = t("previewRunning");
  try {
    const request = await currentCrudRequest();
    const result = await api("crud-preview", { method: "POST", body: JSON.stringify(request) });
    $("crud-source-preview").textContent = result.snippet || "";
    $("crud-diff-preview").textContent = result.diff || "";
    $("crud-impact-preview").textContent = JSON.stringify(result.impact || {}, null, 2);
    $("crud-preview-grid").hidden = false;
    showCrudDiagnostics(result.diagnostics || []);
    crudPreviewValid = Boolean(result.success);
    lastCrudPreviewRequest = crudPreviewValid ? request : null;
    $("crud-apply-button").disabled = !crudPreviewValid;
    $("crud-preview-state").textContent = crudPreviewValid ? t("previewPassed") : t("previewFailed");
    $("crud-preview-state").dataset.kind = crudPreviewValid ? "success" : "error";
  } catch (error) {
    showCrudDiagnostics([]);
    $("crud-preview-state").textContent = error.message;
    $("crud-preview-state").dataset.kind = "error";
  }
}

async function applyCrud() {
  if (!crudPreviewValid) { announce(t("previewRequired"), "warning"); return; }
  try {
    const request = lastCrudPreviewRequest;
    if (!request) throw new Error(t("previewRequired"));
    const result = await api("crud-apply", { method: "POST", body: JSON.stringify(request) });
    const tab = tabs.get(result.path);
    if (tab) {
      tab.saved = result.content; tab.hash = result.hash;
      tab.state = EditorState.create({ doc: result.content });
      if (activePath === result.path && editor) { editor.setState(tab.state); tab.state = editor.state; }
      renderTabs();
    } else await openFile(result.path);
    invalidateCrudPreview();
    await refreshTree();
    announce(t("applySuccess"), "success");
  } catch (error) {
    $("crud-preview-state").textContent = error.message;
    $("crud-preview-state").dataset.kind = "error";
    announce(error.message, "error");
  }
}

function openDatabaseWorkbench() {
  $("editor-stage").hidden = true;
  $("database-workbench").hidden = false;
  $("database-button").classList.add("active");
  if (!databaseMap) void loadDatabaseSchema();
}

function openCodeWorkbench() {
  $("database-workbench").hidden = true;
  $("editor-stage").hidden = false;
  $("database-button").classList.remove("active");
  if (editor) editor.focus();
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
$("database-button").addEventListener("click", openDatabaseWorkbench);
$("code-button").addEventListener("click", openCodeWorkbench);
$("database-refresh").addEventListener("click", loadDatabaseSchema);
$("database-start").addEventListener("click", loadDatabaseSchema);
$("database-filter").addEventListener("input", (event) => { databaseFilter = event.target.value; renderDatabaseTables(); });
$("crud-preview-button").addEventListener("click", previewCrud);
$("crud-apply-button").addEventListener("click", applyCrud);
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
