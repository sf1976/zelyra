import { autocompletion, startCompletion } from "@codemirror/autocomplete";
import { defaultKeymap, history as undoRedoHistory, historyKeymap } from "@codemirror/commands";
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
const editorSessionStorageKey = "zelyra-editor-session-token";
const fragmentToken = location.hash.slice(1);
const token = /^[0-9a-f]{64}$/.test(fragmentToken)
  ? fragmentToken
  : sessionStorage.getItem(editorSessionStorageKey) || "";
if (token) sessionStorage.setItem(editorSessionStorageKey, token);
history.replaceState(null, "", location.pathname);
const $ = (id) => document.getElementById(id);
const status = $("status-message");
const tabs = new Map();
let activePath = null;
let editor = null;
let diagnostics = [];
let liveCheckTimer = null;
let liveCheckSequence = 0;
const symbolCache = new Map();
const symbolRequests = new Map();
let treeFiles = [];
let activeFilter = "";
let projectSearchMode = "text";
let databaseMap = null;
let databaseTables = [];
let selectedDatabaseTable = null;
let databaseFilter = "";
let crudPreviewValid = false;
let lastCrudPreviewRequest = null;
const editorRoot = $("editor-root");
const messages = {
  de: {
    symbolProject: "Projektdeklaration", symbolFunction: "Funktion", symbolEntrypoint: "Programmeinstieg", symbolType: "Typ", symbolTable: "Datenbanktabelle", symbolView: "Ansicht", symbolComponent: "Komponente",
    symbolForm: "Formular", symbolCrud: "CRUD-Ressource", symbolPage: "Seite", symbolApi: "API-Endpunkt",
    symbolTableview: "Abfrageansicht",
    taskFinder: "Aktionen", taskFinderTitle: "Was möchtest du tun?", taskFinderPlaceholder: "Aktion oder Befehl suchen",
    chooseAction: "Aktion auswählen", noMatchingTasks: "Keine passende Aktion gefunden.", taskOpenFile: "Datei öffnen", taskOpenFileHelp: "Projektdatei nach Namen suchen",
    taskNewFile: "Datei erstellen", taskNewFileHelp: "Neue Datei im Projekt anlegen", taskSave: "Aktive Datei speichern",
    taskSaveHelp: "Ungespeicherte Änderungen übernehmen", taskSaveAll: "Alle Dateien speichern", taskSaveAllHelp: "Alle geänderten Dateien übernehmen",
    taskFormat: "Datei formatieren", taskFormatHelp: "Zelyra-Quelldatei kanonisch formatieren", taskCheck: "Projekt prüfen",
    taskCheckHelp: "Compilerdiagnosen für das Projekt anzeigen", taskDatabase: "Datenbank öffnen", taskDatabaseHelp: "Tabellen und Beziehungen ansehen",
    taskRun: "Programm ausführen", taskRunHelp: "main.zyl mit Zelyras Laufzeit starten; konfigurierte Datenbank-, Netzwerk- und Prozesseffekte sind möglich, interaktive Eingabe fehlt",
    projectMap: "Projektkarte", projectMapTitle: "Projektbeziehungen", projectMapOpenSource: "Quelltext öffnen", projectMapLoading: "Compiler-Projektmodell wird geladen …", projectMapUnavailable: "Projektkarte konnte nicht erstellt werden.", projectMapIncomplete: "Die Karte ist unvollständig. Nicht auflösbare oder begrenzte Beziehungen werden nicht geraten.", projectMapEmpty: "Noch keine unterstützten Beziehungen im Projekt.", projectMapModules: "Module", projectMapData: "Daten und Ansichten", projectMapWeb: "Routen und APIs", projectMapLogic: "Funktionen",
    runButton: "Starten", runButtonTitle: "main.zyl ausführen",
    runOutputTitle: "Programmausgabe", runStarting: "main.zyl wird ausgeführt …", runSuccess: "Programm erfolgreich beendet", runFailure: "Programm mit Fehler beendet", runTimeout: "Ausführung nach 15 Sekunden beendet", runOutputTruncated: "Die Ausgabe ist auf 64 KiB je Stream begrenzt.", runNoOutput: "Keine Ausgabe.", runStdout: "AUSGABE", runStderr: "FEHLER", runInputUnavailable: "Studio stellt dem Programm keine interaktive Eingabe bereit.",
    taskHelp: "Kontexthilfe öffnen", taskHelpHelp: "Hilfe zum Begriff am Cursor anzeigen", taskProperties: "Dateieigenschaften öffnen",
    taskPropertiesHelp: "Informationen und Aktionen zur aktuellen Datei anzeigen", taskSearch: "Text suchen", taskSearchHelp: "Im aktuellen Dokument suchen",
    taskGoToDefinition: "Zur Definition springen", taskGoToDefinitionHelp: "Zur Funktion oder zum Typ am Cursor springen (F12)",
    taskFindReferences: "Bezeichnerverwendungen finden", taskFindReferencesHelp: "Exakte Zelyra-Bezeichner im Projekt finden (Umschalt+F12)",
    definitionNotFound: "Keine passende Definition im Projekt gefunden.", definitionSaveFirst: "Speichere die Datei vor dem Sprung, damit die Quellposition stimmt.", definitionOpened: "Definition geöffnet: {0}",
    projectSearch: "Projekt durchsuchen", projectSearchTitle: "Projekt durchsuchen", projectSearchHelp: "Alle gespeicherten Zelyra-Dateien durchsuchen", projectSearchPlaceholder: "Text in Zelyra-Dateien suchen", projectSearchResults: "Suchergebnisse", projectSearchEmpty: "Keine Treffer gefunden.", projectSearchLoading: "Projekt wird durchsucht …", projectSearchSaveFirst: "Speichere offene Änderungen, bevor du das Projekt durchsuchst.", projectSearchTruncated: "Die Trefferliste ist wegen einer Suchgrenze unvollständig.", projectSearchSubmit: "Suchen",
    referencesTitle: "Bezeichnerverwendungen", referencesPlaceholder: "Zelyra-Bezeichner suchen",
    localParameter: "Parameter", localValue: "Lokaler Wert", loopValue: "Schleifenwert", matchValue: "Musterwert",
    assistant: "Hilfe und Eigenschaften", assistantTabs: "Studio-Seitenleiste", taskFinderResults: "Aktionen", helpTab: "Hilfe", propertiesTab: "Eigenschaften", contextHelp: "KONTEXT-HILFE",
    selectLanguageItem: "Wähle einen Zelyra-Begriff", contextHelpHint: "Bewege den Cursor auf einen Begriff. F1 öffnet diese Hilfe.",
    fileProperties: "DATEIEIGENSCHAFTEN", fileType: "Dateityp", filePath: "Projektpfad", fileStatus: "Status",
    sourceKind: "Zelyra-Quelltext", configFile: "Projektkonfiguration", otherFile: "Projektdatei", savedState: "Gespeichert", unsavedState: "Ungespeichert",
    subtitle: "Lokale Entwicklungsumgebung", local: "LOKAL", files: "DATEIEN", newFile: "Neue Datei",
    refresh: "Aktualisieren", filterFiles: "Dateien filtern", saveAll: "Alle speichern", save: "Speichern",
    renameFile: "Umbenennen", deleteFile: "Löschen", renamePrompt: "Neuer Projektpfad (zum Beispiel src/customer-new.zyl)",
    renameConfirm: "Datei {0} umbenennen? Importverweise werden nicht angepasst.", deleteConfirm: "Datei {0} endgültig löschen?",
    fileRenamed: "Datei umbenannt: {0}", fileDeleted: "Datei gelöscht: {0}",
    switchLanguage: "Sprache wechseln",
    format: "Formatieren", check: "Projekt prüfen", welcome: "Willkommen in Zelyra Studio",
    welcomeHint: "Öffne links eine Datei und entwickle direkt in deinem Projekt.", problems: "Probleme",
    noProblems: "Keine Probleme gefunden. Dein Projekt ist bereit.", close: "Panel schließen", loadingFiles: "Dateien werden geladen",
    toggleExplorer: "Dateiübersicht umschalten", search: "Suchen", handbook: "Handbuch", projectFiles: "Projektdateien",
    openFile: "Datei öffnen", complete: "Vervollständigen", completeHelp: "Codevorschläge anzeigen (Strg+Leertaste)", ready: "Bereit", localMode: "Lokaler Modus",
    noDocument: "Kein Dokument geöffnet", opened: "{0} geöffnet", unsaved: "Ungespeicherte Änderungen",
    saved: "{0} gespeichert", checkRunning: "Projekt wird geprüft …", checkPassed: "Projektprüfung erfolgreich",
    problemsFound: "{0} Problem(e) gefunden", formatRunning: "Formatiere …", formatted: "{0} formatiert",
    createPrompt: "Neuer Dateipfad im Projekt (zum Beispiel src/customer.zyl)",
    closeUnsaved: "{0} enthält ungespeicherte Änderungen. Tab trotzdem schließen?",
    filesCount: "{0} Dateien", chooseFile: "Datei öffnen", database: "Datenbank", databaseOpenTable: "Tabelle auswählen",
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
    symbolProject: "Project declaration", symbolFunction: "Function", symbolEntrypoint: "Program entry point", symbolType: "Type", symbolTable: "Database table", symbolView: "View", symbolComponent: "Component",
    symbolForm: "Form", symbolCrud: "CRUD resource", symbolPage: "Page", symbolApi: "API endpoint",
    symbolTableview: "Query view",
    taskFinder: "Actions", taskFinderTitle: "What would you like to do?", taskFinderPlaceholder: "Search actions or commands",
    chooseAction: "Choose action", noMatchingTasks: "No matching action found.", taskOpenFile: "Open file", taskOpenFileHelp: "Find a project file by name",
    taskNewFile: "Create file", taskNewFileHelp: "Add a new file to the project", taskSave: "Save active file",
    taskSaveHelp: "Write the current changes", taskSaveAll: "Save all files", taskSaveAllHelp: "Write all changed files",
    taskFormat: "Format file", taskFormatHelp: "Format a Zelyra source file", taskCheck: "Check project",
    taskCheckHelp: "Show compiler diagnostics for the project", taskDatabase: "Open database", taskDatabaseHelp: "Inspect tables and relationships",
    taskRun: "Run program", taskRunHelp: "Run main.zyl with Zelyra; configured database, network, and process effects are possible, and interactive input is unavailable",
    projectMap: "Project map", projectMapTitle: "Project relationships", projectMapOpenSource: "Open source location", projectMapLoading: "Loading the compiler project model …", projectMapUnavailable: "Could not create the project map.", projectMapIncomplete: "The map is incomplete. Unresolved or limited relationships are not guessed.", projectMapEmpty: "No supported relationships in this project yet.", projectMapModules: "Modules", projectMapData: "Data and views", projectMapWeb: "Routes and APIs", projectMapLogic: "Functions",
    runButton: "Run", runButtonTitle: "Run main.zyl",
    runOutputTitle: "Program output", runStarting: "Running main.zyl …", runSuccess: "Program finished successfully", runFailure: "Program finished with an error", runTimeout: "Execution stopped after 15 seconds", runOutputTruncated: "Output is limited to 64 KiB per stream.", runNoOutput: "No output.", runStdout: "OUTPUT", runStderr: "ERROR", runInputUnavailable: "Studio does not provide interactive input to the program.",
    taskHelp: "Open contextual help", taskHelpHelp: "Show help for the term at the cursor", taskProperties: "Open file properties",
    taskPropertiesHelp: "Show information and actions for the active file", taskSearch: "Find text", taskSearchHelp: "Search in the active document",
    taskGoToDefinition: "Go to definition", taskGoToDefinitionHelp: "Go to the function or type at the cursor (F12)",
    taskFindReferences: "Find identifier occurrences", taskFindReferencesHelp: "Find exact Zelyra identifiers across the project (Shift+F12)",
    definitionNotFound: "No matching project definition was found.", definitionSaveFirst: "Save this file before jumping so source positions stay accurate.", definitionOpened: "Opened definition: {0}",
    projectSearch: "Search project", projectSearchTitle: "Search project", projectSearchHelp: "Search all saved Zelyra files", projectSearchPlaceholder: "Search text in Zelyra files", projectSearchResults: "Search results", projectSearchEmpty: "No matches found.", projectSearchLoading: "Searching the project …", projectSearchSaveFirst: "Save open changes before searching the project.", projectSearchTruncated: "The result list is incomplete because a search limit was reached.", projectSearchSubmit: "Search",
    referencesTitle: "Identifier occurrences", referencesPlaceholder: "Find a Zelyra identifier",
    localParameter: "Parameter", localValue: "Local value", loopValue: "Loop value", matchValue: "Match value",
    assistant: "Help and properties", assistantTabs: "Studio sidebar", taskFinderResults: "Actions", helpTab: "Help", propertiesTab: "Properties", contextHelp: "CONTEXT HELP",
    selectLanguageItem: "Select a Zelyra term", contextHelpHint: "Place the cursor on a term. Press F1 to open this help.",
    fileProperties: "FILE PROPERTIES", fileType: "File type", filePath: "Project path", fileStatus: "Status",
    sourceKind: "Zelyra source", configFile: "Project configuration", otherFile: "Project file", savedState: "Saved", unsavedState: "Unsaved",
    subtitle: "Local development environment", local: "LOCAL", files: "FILES", newFile: "New file",
    refresh: "Refresh", filterFiles: "Filter files", saveAll: "Save all", save: "Save",
    renameFile: "Rename", deleteFile: "Delete", renamePrompt: "New project path (for example src/customer-new.zyl)",
    renameConfirm: "Rename {0}? Import references will not be updated.", deleteConfirm: "Permanently delete {0}?",
    fileRenamed: "File renamed: {0}", fileDeleted: "File deleted: {0}",
    switchLanguage: "Switch language",
    format: "Format", check: "Check project", welcome: "Welcome to Zelyra Studio",
    welcomeHint: "Open a file on the left and work directly in your project.", problems: "Problems",
    noProblems: "No problems found. Your project is ready.", close: "Close panel", loadingFiles: "Loading files",
    toggleExplorer: "Toggle file explorer", search: "Search", handbook: "Handbook", projectFiles: "Project files",
    openFile: "Open file", complete: "Complete", completeHelp: "Show code suggestions (Ctrl+Space)", ready: "Ready", localMode: "Local mode",
    noDocument: "No document open", opened: "Opened {0}", unsaved: "Unsaved changes",
    saved: "Saved {0}", checkRunning: "Checking project …", checkPassed: "Project check passed",
    problemsFound: "Found {0} problem(s)", formatRunning: "Formatting …", formatted: "Formatted {0}",
    createPrompt: "New project file path (for example src/customer.zyl)",
    closeUnsaved: "{0} has unsaved changes. Close this tab anyway?",
    filesCount: "{0} files", chooseFile: "Open file", database: "Database", databaseOpenTable: "Select table",
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
  if (projectSearchMode === "references") {
    $("project-search-title").textContent = t("referencesTitle");
    $("project-search-input").placeholder = t("referencesPlaceholder");
  }
  localStorage.setItem("zelyra-editor-language", language);
  renderTree();
  if (databaseMap) {
    renderDatabaseTables();
    if (selectedDatabaseTable) selectDatabaseTable(selectedDatabaseTable.name);
    else renderDatabaseGraph();
  }
  if (editor && activePath) renderContextHelp(editor.state, activePath);
  if (!$("studio-assistant").hidden) renderFileProperties(activePath);
}

const keywords = new Set((
  "fn let mut if else match for in while loop break continue return type record " +
  "table view component layout slot page api form crud tableview auth database " +
  "sql uses capability requires ensures invariant true false null none some ok err"
).split(/\s+/));
const languageHelp = {
  fn: ["Funktion", "Definiert einen benannten, wiederverwendbaren Ablauf.", "fn begruessung(name: Text) -> Text", "return \"Hallo \" + name"],
  let: ["Wert", "Legt einen veränderlichen Wert im aktuellen Gültigkeitsbereich an.", "let anzahl = 0", "anzahl = anzahl + 1"],
  mut: ["Veränderlicher Wert", "Kennzeichnet einen Wert, der später neu gesetzt werden darf.", "let mut status = \"offen\"", "status = \"bezahlt\""],
  table: ["Datenbanktabelle", "Beschreibt Tabellenfelder und ihre Eigenschaften in der Projektstruktur.", "table kunden {\n  name: Text\n  email: Text unique\n}", "Studio kann die erkannte Tabellenstruktur anzeigen und CRUD vorbereiten."],
  form: ["Eingabeformular", "Erstellt ein Formular aus definierten Feldern und bindet es an eine Ressource.", "form kunde {\n  name: Text required\n  email: Text\n}", "Felder prüfen und CRUD-Ansicht über das Datenbank Studio ergänzen."],
  crud: ["Datenansicht", "Verknüpft eine Ressource mit typischen Listen- und Bearbeitungsabläufen.", "crud kunden {\n  title: \"Kunden\"\n}", "Das Studio kann eine geprüfte Vorschau mit Auswirkungen auf Routen zeigen."],
  view: ["Ansicht", "Beschreibt eine serverseitig gerenderte Seite.", "view kunden {\n  title: \"Kunden\"\n}", "Die Eigenschaften richten sich nach dem Ansichtsblock."],
  route: ["Route", "Ordnet eine Adresse und HTTP-Methode einem Handler zu.", "route GET /kunden { ... }", "Doppelte Routen erkennt die Projektprüfung."],
  component: ["Komponente", "Bündelt wiederverwendbare Darstellung und Verhalten.", "component Hinweis { ... }", "Eine Komponente kann in Ansichten wiederverwendet werden."],
  if: ["Bedingung", "Führt einen Block aus, wenn der Ausdruck wahr ist.", "if kunde.aktiv { ... }", "Mit else lässt sich ein Alternativpfad beschreiben."],
  for: ["Wiederholung", "Verarbeitet jedes Element einer Liste in einem Block.", "for artikel in artikel { ... }", "Der Name nach `for` bezeichnet das aktuelle Element."],
  match: ["Fallunterscheidung", "Wählt einen passenden Zweig anhand eines Wertes.", "match status { ... }", "Hilft, alle bekannten Fälle sichtbar abzudecken."],
  record: ["Datensatztyp", "Definiert zusammengehörige typisierte Werte.", "record Adresse {\n  ort: Text\n  plz: Text\n}", "Datensatztypen machen Datenformen im Code sichtbar."],
  type: ["Typ", "Definiert einen eigenen Typnamen oder Typverbund.", "type KundenId = Zahl", "Typen helfen der IDE, Werte und Fehler einzuordnen."],
  sql: ["SQL-Abfrage", "Führt eine parameterisierte Datenbankabfrage im deklarierten Kontext aus.", "sql \"SELECT ... WHERE id = ?\"", "Nutze Parameter für Werte; füge Eingaben nicht in SQL-Text ein."],
  requires: ["Vorbedingung", "Beschreibt, was vor einem Funktionsaufruf gelten muss.", "requires betrag >= 0", "Vorbedingungen werden bei der Projektprüfung berücksichtigt."],
  ensures: ["Nachbedingung", "Beschreibt eine Garantie nach erfolgreicher Ausführung.", "ensures result >= 0", "Hilft, Absichten für Menschen und Werkzeuge sichtbar zu machen."],
  invariant: ["Invariante", "Formuliert eine Bedingung, die dauerhaft gelten soll.", "invariant saldo >= 0", "Geeignet für wichtige Regeln eines Datenmodells."],
  api: ["API-Endpunkt", "Deklariert einen typisierten HTTP-Endpunkt mit Anfrage und Antwort.", "api GET \"/customers/{id}\" { handler get_customer }", "Der Compiler prüft Eingaben, Ausgaben und Handler."],
  page: ["Webseite", "Verknüpft einen URL-Pfad mit einer HTML-Ansicht.", "page \"/hello/{name}\" { html { … } }", "Routenparameter werden im Seiteninhalt verfügbar."],
  database: ["Datenbankverbindung", "Deklariert das Datenbank-Backend und den logischen Verbindungsnamen.", "database main { engine: mariadb }", "Die Zugangsdaten kommen aus der Prozessumgebung, nicht aus dem Quelltext."],
  tableview: ["Abfrageansicht", "Stellt ausgewählte Abfrageergebnisse als typisierte Ansicht bereit.", "tableview Customers { source sql<Customer[]> { … } }", "Die Datenabfrage und ihr Ergebnistyp werden geprüft."],
  auth: ["Authentifizierung", "Legt die Tabellen und Routen der Anmeldung und Berechtigungsverwaltung fest.", "auth users { table: users sessions: auth_sessions }", "Authentifizierungszugriff und Tabellen müssen zusammenpassen."],
  uses: ["Capability-Anforderung", "Deklariert, welche Laufzeitfähigkeit eine Funktion verwenden darf.", "fn main() uses Console { … }", "Auch eine Projektfreigabe kann erforderlich sein."],
  capability: ["Explizite Laufzeitfähigkeit", "Begrenzt, welche externen Aktionen ein Projekt ausführen darf.", "[capabilities]\nconsole = true", "Eine Freigabe im Projekt hebt keine Aufrufregeln des Compilers auf."],
};
const languageHelpEn = {
  fn: ["function", "Defines a named, reusable operation.", "fn greet(name: Text) -> Text", "return \"Hello \" + name"],
  let: ["value", "Creates a value in the current scope.", "let count = 0", "count = count + 1"],
  mut: ["mutable value", "Marks a value that may be assigned again later.", "let mut status = \"open\"", "status = \"paid\""],
  table: ["database table", "Describes fields and their properties in the project schema.", "table customers {\n  name: Text\n  email: Text unique\n}", "Studio can inspect the declared table and prepare a checked CRUD preview."],
  form: ["input form", "Creates a form from declared fields and binds it to a resource.", "form customer {\n  name: Text required\n  email: Text\n}", "Use the database studio to inspect fields and prepare a CRUD view."],
  crud: ["data view", "Connects a resource to common list and edit workflows.", "crud customers {\n  title: \"Customers\"\n}", "Studio can preview the source and show affected routes."],
  view: ["view", "Describes a server-rendered page.", "view customers {\n  title: \"Customers\"\n}", "Available properties depend on the view block."],
  route: ["route", "Maps an address and HTTP method to a handler.", "route GET /customers { ... }", "Project checks report duplicate routes."],
  component: ["component", "Groups reusable presentation and behavior.", "component Notice { ... }", "Components can be reused in views."],
  if: ["condition", "Runs a block when its expression is true.", "if customer.active { ... }", "Use else to describe an alternative path."],
  for: ["iteration", "Processes each item in a list inside a block.", "for item in items { ... }", "The name after `for` refers to the current item."],
  match: ["pattern match", "Selects a branch based on a value.", "match status { ... }", "Makes known cases visible in the source."],
  record: ["record type", "Defines a group of related, typed values.", "record Address {\n  city: Text\n  zip: Text\n}", "Record types make data shapes explicit."],
  type: ["type", "Defines a type alias or a named type.", "type CustomerId = Number", "Types help Studio and the compiler identify values and errors."],
  sql: ["SQL query", "Runs a parameterized database query in its declared context.", "sql \"SELECT ... WHERE id = ?\"", "Use parameters for values; do not concatenate user input into SQL."],
  requires: ["precondition", "States what must be true before a function call.", "requires amount >= 0", "Preconditions are included in project checks."],
  ensures: ["postcondition", "States a guarantee after successful execution.", "ensures result >= 0", "Makes intended outcomes clear to people and tools."],
  invariant: ["invariant", "States a condition that should remain true.", "invariant balance >= 0", "Useful for important rules in a data model."],
  api: ["API endpoint", "Declares a typed HTTP endpoint with a request and response.", "api GET \"/customers/{id}\" { handler get_customer }", "The compiler checks inputs, outputs, and the handler."],
  page: ["web page", "Connects a URL path to an HTML view.", "page \"/hello/{name}\" { html { … } }", "Route parameters are available inside the page content."],
  database: ["database connection", "Declares the backend and logical connection name.", "database main { engine: mariadb }", "Credentials come from the process environment, not source code."],
  tableview: ["query view", "Presents selected query results as a typed view.", "tableview Customers { source sql<Customer[]> { … } }", "The query and its result type are checked."],
  auth: ["authentication", "Declares the tables and routes for sign-in and permission management.", "auth users { table: users sessions: auth_sessions }", "Authentication access and tables must agree."],
  uses: ["capability requirement", "Declares which runtime capability a function may use.", "fn main() uses Console { … }", "The project may also need to grant that capability."],
  capability: ["explicit runtime capability", "Limits which external actions a project can perform.", "[capabilities]\nconsole = true", "A project grant does not remove compiler call rules."],
};
const languageOptions = {
  de: {
    fn: ["Parameter mit Typ", "Rückgabetyp", "uses Capability", "requires Vorbedingung", "ensures Nachbedingung"],
    table: ["typisierte Felder", "required", "unique", "primary auto", "default Wert", "index { Feld }"],
    form: ["fields { Feldnamen }", "field Name: Typ", "label", "required", "max", "widget"],
    crud: ["title", "view { fields { … } }", "list", "detail", "create_form", "edit_form"],
    route: ["HTTP-Methode und Pfad", "handler Funktionsname", "input", "output", "errors"],
    api: ["HTTP-Methode und Pfad", "handler", "input", "output", "errors", "version", "rate_limit"],
    database: ["engine: mariadb | mysql | postgres | sqlite", "database: Name"],
    page: ["HTML-Inhalt in html { … }", "Routenparameter im Seiteninhalt"],
    tableview: ["source sql<Typ> { … }", "strukturierte Feldauswahl"],
    auth: ["table", "sessions", "permissions", "roles", "admin_path"],
    uses: ["explizite Capabilitys, z. B. Database oder Console"],
    capability: ["Projektfreigabe in [capabilities] der zelyra.toml"],
  },
  en: {
    fn: ["typed parameters", "return type", "uses capability", "requires precondition", "ensures postcondition"],
    table: ["typed fields", "required", "unique", "primary auto", "default value", "index { field }"],
    form: ["fields { field names }", "field Name: Type", "label", "required", "max", "widget"],
    crud: ["title", "view { fields { … } }", "list", "detail", "create_form", "edit_form"],
    route: ["HTTP method and path", "handler function name", "input", "output", "errors"],
    api: ["HTTP method and path", "handler", "input", "output", "errors", "version", "rate_limit"],
    database: ["engine: mariadb | mysql | postgres | sqlite", "database: name"],
    page: ["HTML content inside html { … }", "route parameters in page content"],
    tableview: ["source sql<Type> { … }", "structured field selection"],
    auth: ["table", "sessions", "permissions", "roles", "admin_path"],
    uses: ["explicit capabilities, for example Database or Console"],
    capability: ["project grant in [capabilities] in zelyra.toml"],
  },
};
const zelyraLanguage = StreamLanguage.define({
  startState: () => ({ inBlockComment: false }),
  languageData: { commentTokens: { line: "#", block: { open: "##", close: "##" } } },
  token(stream, state) {
    if (stream.string.trim() === "##") {
      stream.skipToEnd();
      state.inBlockComment = !state.inBlockComment;
      return "comment";
    }
    if (state.inBlockComment) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.eatSpace()) return null;
    if (stream.match("#")) { stream.skipToEnd(); return "comment"; }
    if (stream.match("//")) { stream.skipToEnd(); return "comment"; }
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

function tabState(path, tab) { return path === activePath && editor ? editor.state : tab?.state; }
function isDirty(tab, path) { return tab && tabState(path, tab).doc.toString() !== tab.saved; }
function dirtyCount() { return [...tabs].filter(([path, tab]) => isDirty(tab, path)).length; }

function renderTabs() {
  const bar = $("tabs");
  bar.replaceChildren();
  for (const [path, tab] of tabs) {
    const button = document.createElement("button");
    button.className = `tab${path === activePath ? " active" : ""}`;
    button.title = path;
    const dot = document.createElement("span");
    dot.className = `tab-state${isDirty(tab, path) ? " dirty" : ""}`;
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
  $("save-button").disabled = !activePath || !isDirty(tabs.get(activePath), activePath);
  $("complete-button").disabled = !activePath;
  $("save-all-button").disabled = dirtyCount() === 0;
  const protectedFile = activePath === "main.zyl" || activePath === "zelyra.toml";
  $("rename-file-button").disabled = !activePath || protectedFile;
  $("delete-file-button").disabled = !activePath || protectedFile;
  if (!$("studio-assistant").hidden) renderFileProperties(activePath);
}

function completionSource(context) {
  const word = context.matchBefore(/[\w]+/);
  if (!word && !context.explicit) return null;
  const source = context.state.doc.toString();
  return Promise.all([
    loadProjectSymbols(activePath),
    loadLocalSymbols(activePath, source, context.pos),
  ]).then(([symbols, locals]) => {
    const localOptions = [...new Map(locals.map((item) => [item.label, item])).values()].map((item) => ({
      label: item.label,
      type: "variable",
      detail: `${t(item.detail === "parameter" ? "localParameter" : item.detail === "loop value" ? "loopValue" : item.detail === "match value" ? "matchValue" : "localValue")}${item.valueType ? ` · ${item.valueType}` : ""}`,
    }));
    const localNames = new Set(localOptions.map((item) => item.label));
    const projectOptions = symbols.filter((symbol) => !localNames.has(symbol.label)).map((symbol) => ({
      label: symbol.label,
      type: symbol.kind,
      detail: symbol.detail ? t(`symbol${symbol.detail[0].toUpperCase()}${symbol.detail.slice(1)}`) : t("symbolProject"),
    }));
    if (!localNames.has("main") && !projectOptions.some((item) => item.label === "main")) {
      projectOptions.push({ label: "main", type: "function", detail: t("symbolEntrypoint") });
    }
    return {
      from: word ? word.from : context.pos,
      options: [
        ...[...keywords].map((label) => ({ label, type: "keyword" })),
        ...localOptions,
        ...projectOptions,
      ],
    };
  });
}

function loadLocalSymbols(path, source, offset) {
  if (!path?.endsWith(".zyl")) return Promise.resolve([]);
  return api("locals", { method: "POST", body: JSON.stringify({ path, source, offset }) })
    .then((result) => result.symbols || [])
    .catch(() => []);
}

function loadProjectSymbols(path) {
  if (!path?.endsWith(".zyl")) return Promise.resolve([]);
  const cached = symbolCache.get(path);
  if (cached && cached.expires > Date.now()) return Promise.resolve(cached.symbols);
  if (symbolRequests.has(path)) return symbolRequests.get(path);
  const request = api(`symbols?path=${encodeURIComponent(path)}`)
    .then((result) => result.symbols || [])
    .catch(() => [])
    .then((symbols) => {
      symbolCache.set(path, { symbols, expires: Date.now() + 2_000 });
      symbolRequests.delete(path);
      return symbols;
    });
  symbolRequests.set(path, request);
  return request;
}

function selectedLanguageWord(state) {
  const selection = state.selection.main;
  const line = state.doc.lineAt(selection.head).text;
  const offset = selection.head - state.doc.lineAt(selection.head).from;
  const match = [...line.matchAll(/[A-Za-z_][\w]*/g)].find((item) => item.index <= offset && item.index + item[0].length >= offset);
  return match?.[0]?.toLowerCase() || "";
}

function renderContextHelp(state, path) {
  if ($("studio-assistant").hidden || !path?.endsWith(".zyl")) return;
  const word = selectedLanguageWord(state);
  const help = (language === "en" ? languageHelpEn[word] : null) || languageHelp[word];
  $("assistant-help-title").textContent = help ? `${word} · ${help[0]}` : (word || t("selectLanguageItem"));
  $("assistant-help-description").textContent = help ? help[1] : t("contextHelpHint");
  const details = $("assistant-help-details"); details.replaceChildren();
  if (!help) return;
  const syntaxTitle = document.createElement("strong"); syntaxTitle.textContent = language === "de" ? "Schreibweise" : "Syntax";
  const syntax = document.createElement("pre"); syntax.textContent = help[2];
  const exampleTitle = document.createElement("strong"); exampleTitle.textContent = language === "de" ? "Hinweis" : "Tip";
  const example = document.createElement("p"); example.textContent = help[3];
  details.append(syntaxTitle, syntax);
  const options = languageOptions[language][word];
  if (options?.length) {
    const optionsTitle = document.createElement("strong"); optionsTitle.textContent = language === "de" ? "Optionen" : "Options";
    const list = document.createElement("ul");
    for (const option of options) { const item = document.createElement("li"); item.textContent = option; list.append(item); }
    details.append(optionsTitle, list);
  }
  details.append(exampleTitle, example);
}

function renderFileProperties(path) {
  if (!path || $("studio-assistant").hidden) return;
  const tab = tabs.get(path);
  const extension = path.split(".").pop()?.toLowerCase() || "";
  const kind = extension === "zyl" ? t("sourceKind") : path === "zelyra.toml" ? t("configFile") : t("otherFile");
  $("assistant-file-name").textContent = path.split("/").pop();
  const details = $("assistant-file-details"); details.replaceChildren();
  for (const [label, value] of [[t("fileType"), kind], [t("filePath"), path], [t("fileStatus"), tab && isDirty(tab, path) ? t("unsavedState") : t("savedState")]]) {
    const term = document.createElement("dt"); term.textContent = label;
    const description = document.createElement("dd"); description.textContent = value;
    details.append(term, description);
  }
}

function showAssistantTab(tab) {
  for (const [name, id] of [["help", "assistant-help-tab"], ["properties", "assistant-properties-tab"]]) {
    const button = $(id);
    const selected = tab === name;
    button.classList.toggle("selected", selected);
    button.setAttribute("aria-selected", String(selected));
    button.tabIndex = selected ? 0 : -1;
  }
  $("assistant-help-view").hidden = tab !== "help";
  $("assistant-properties-view").hidden = tab !== "properties";
  if (tab === "properties") renderFileProperties(activePath);
  else if (editor) renderContextHelp(editor.state, activePath);
}

function toggleAssistant(tab = "help") {
  const panel = $("studio-assistant");
  panel.hidden = !panel.hidden;
  if (!panel.hidden) showAssistantTab(tab);
  requestAnimationFrame(() => editor?.requestMeasure());
}

const studioTasks = [
  ["taskOpenFile", "taskOpenFileHelp", ["file", "open"], () => { const query = prompt(t("chooseFile")); if (query) { const match = treeFiles.find((file) => file.toLowerCase().includes(query.toLowerCase())); if (match) void openFile(match); } }],
  ["taskNewFile", "taskNewFileHelp", ["file", "create", "new"], () => void createFile()],
  ["taskSave", "taskSaveHelp", ["save", "write"], () => void saveActive()],
  ["taskSaveAll", "taskSaveAllHelp", ["save", "all"], () => void saveAll()],
  ["taskFormat", "taskFormatHelp", ["format", "formatter"], () => void formatActive()],
  ["taskCheck", "taskCheckHelp", ["check", "compiler", "error", "diagnostic"], () => void checkProject()],
  ["taskRun", "taskRunHelp", ["run", "execute", "start program"], () => void runProject()],
  ["projectMap", "projectMapTitle", ["map", "diagram", "module", "route", "relationship"], () => void openProjectMap()],
  ["taskDatabase", "taskDatabaseHelp", ["database", "table", "relation", "schema"], () => openDatabaseWorkbench()],
  ["taskHelp", "taskHelpHelp", ["help", "documentation", "f1"], () => toggleAssistant("help")],
  ["taskProperties", "taskPropertiesHelp", ["properties", "file", "settings"], () => toggleAssistant("properties")],
  ["taskSearch", "taskSearchHelp", ["find", "search", "text"], () => findInDocument()],
  ["taskGoToDefinition", "taskGoToDefinitionHelp", ["definition", "symbol", "function", "type", "f12"], () => void goToDefinition()],
  ["taskFindReferences", "taskFindReferencesHelp", ["references", "usages", "occurrences", "symbol", "shift f12"], () => void findIdentifierOccurrences()],
  ["projectSearch", "projectSearchHelp", ["project", "search", "find", "all files"], () => openProjectSearch()],
];
let selectedTaskIndex = 0;

function renderTaskFinder() {
  const query = $("task-finder-input").value.trim().toLocaleLowerCase();
  const results = $("task-finder-results"); results.replaceChildren();
  const matches = studioTasks.filter(([title, description, terms]) =>
    [t(title), t(description), ...terms].join(" ").toLocaleLowerCase().includes(query));
  $("task-finder-empty").hidden = matches.length > 0;
  if (!matches.length) {
    selectedTaskIndex = -1;
    $("task-finder-input").removeAttribute("aria-activedescendant");
    return;
  }
  selectedTaskIndex = Math.max(0, Math.min(selectedTaskIndex, matches.length - 1));
  matches.forEach(([title, description, , action], index) => {
    const row = document.createElement("button"); row.type = "button"; row.className = "task-finder-item";
    row.id = `task-finder-option-${index}`;
    row.tabIndex = -1;
    row.setAttribute("role", "option"); row.setAttribute("aria-selected", String(index === selectedTaskIndex));
    const name = document.createElement("strong"); name.textContent = t(title);
    const hint = document.createElement("span"); hint.textContent = t(description);
    row.append(name, hint); row.addEventListener("click", () => runStudioTask(action)); results.append(row);
  });
  $("task-finder-input").setAttribute("aria-activedescendant", `task-finder-option-${selectedTaskIndex}`);
}

function runStudioTask(action) {
  $("task-finder").close();
  action();
}

function openTaskFinder() {
  const dialog = $("task-finder");
  if (!dialog.open) dialog.showModal();
  $("task-finder-input").setAttribute("aria-expanded", "true");
  $("task-finder-input").value = ""; selectedTaskIndex = 0; renderTaskFinder();
  $("task-finder-input").focus();
}

function findInDocument() {
  if (!editor) return;
  editor.focus();
  import("@codemirror/search").then(({ openSearchPanel }) => openSearchPanel(editor));
}

function openProjectSearch() {
  projectSearchMode = "text";
  if (dirtyCount()) { announce(t("projectSearchSaveFirst"), "warning"); return; }
  const dialog = $("project-search");
  if (!dialog.open) dialog.showModal();
  $("project-search-title").textContent = t("projectSearchTitle");
  $("project-search-input").placeholder = t("projectSearchPlaceholder");
  $("project-search-input").value = "";
  $("project-search-results").replaceChildren();
  $("project-search-empty").hidden = true;
  $("project-search-input").focus();
}

function findIdentifierOccurrences() {
  if (!editor || !activePath?.endsWith(".zyl")) {
    announce(t("definitionNotFound"), "warning");
    return;
  }
  if (dirtyCount()) {
    announce(t("projectSearchSaveFirst"), "warning");
    return;
  }
  const selection = editor.state.selection.main;
  const wordRange = selection.empty ? editor.state.wordAt(selection.head) : selection;
  const name = wordRange ? editor.state.sliceDoc(wordRange.from, wordRange.to).trim() : "";
  if (!name) {
    announce(t("definitionNotFound"), "warning");
    return;
  }
  projectSearchMode = "references";
  const dialog = $("project-search");
  if (!dialog.open) dialog.showModal();
  $("project-search-title").textContent = t("referencesTitle");
  $("project-search-input").placeholder = t("referencesPlaceholder");
  $("project-search-input").value = name;
  $("project-search-results").replaceChildren();
  $("project-search-empty").hidden = true;
  void performProjectSearch();
}

async function performProjectSearch() {
  if (dirtyCount()) { announce(t("projectSearchSaveFirst"), "warning"); $("project-search").close(); return; }
  const query = $("project-search-input").value.trim();
  if (!query) return;
  const results = $("project-search-results");
  results.replaceChildren();
  const empty = $("project-search-empty");
  empty.textContent = t("projectSearchLoading");
  empty.hidden = false;
  try {
    const response = projectSearchMode === "references"
      ? await api(`references?name=${encodeURIComponent(query)}`)
      : await api(`search?q=${encodeURIComponent(query)}`);
    const matches = response.matches || [];
    empty.textContent = t("projectSearchEmpty");
    empty.hidden = matches.length > 0;
    for (const match of matches) {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "task-finder-item";
      const file = document.createElement("strong");
      file.textContent = `${match.path}:${match.line}:${match.column}`;
      const snippet = document.createElement("span");
      snippet.textContent = match.snippet;
      row.append(file, snippet);
      row.addEventListener("click", () => {
        void openFile(match.path).then(() => {
          if (activePath !== match.path || !editor) return;
          const lineNumber = Math.max(1, Math.min(match.line, editor.state.doc.lines));
          const line = editor.state.doc.line(lineNumber);
          const position = Math.min(line.from + Math.max(0, match.column - 1), line.to);
          editor.dispatch({ selection: { anchor: position }, scrollIntoView: true });
          editor.focus();
          $("project-search").close();
        });
      });
      results.append(row);
    }
    if (response.truncated) {
      const note = document.createElement("p");
      note.className = "task-finder-empty";
      note.textContent = t("projectSearchTruncated");
      results.append(note);
    }
  } catch (error) {
    empty.textContent = error.message;
    empty.hidden = false;
  }
}

async function goToDefinition() {
  if (!editor || !activePath?.endsWith(".zyl")) return;
  if (isDirty(tabs.get(activePath), activePath)) {
    announce(t("definitionSaveFirst"), "warning");
    return;
  }
  const wordRange = editor.state.wordAt(editor.state.selection.main.head);
  const word = wordRange ? editor.state.sliceDoc(wordRange.from, wordRange.to) : "";
  if (!word) { announce(t("definitionNotFound"), "warning"); return; }
  try {
    const symbols = await loadProjectSymbols(activePath);
    const symbol = symbols.find((item) => item.label === word && item.path === activePath && Number.isInteger(item.position))
      || symbols.find((item) => item.label === word && item.path && Number.isInteger(item.position));
    if (!symbol) { announce(t("definitionNotFound"), "warning"); return; }
    const targetTab = tabs.get(symbol.path);
    if (targetTab && isDirty(targetTab, symbol.path)) {
      announce(t("definitionSaveFirst"), "warning");
      return;
    }
    await openFile(symbol.path);
    if (activePath !== symbol.path || !editor) return;
    editor.dispatch({ selection: { anchor: symbol.position, head: symbol.position + word.length }, scrollIntoView: true });
    editor.focus();
    announce(t("definitionOpened", word), "success");
  } catch (error) { announce(error.message, "error"); }
}

function createView(path, state) {
  return new EditorView({
    state,
    parent: editorRoot,
    extensions: [
      lineNumbers(), highlightActiveLineGutter(), foldGutter(), undoRedoHistory(), drawSelection(),
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
        { key: "Ctrl-Space", run: startCompletion },
        { key: "Mod-Space", run: startCompletion },
        { key: "F12", run: () => { void goToDefinition(); return true; } },
        { key: "Shift-F12", run: () => { void findIdentifierOccurrences(); return true; } },
        { key: "F1", run: () => { toggleAssistant("help"); return true; } },
        ...defaultKeymap, ...historyKeymap, ...searchKeymap, ...foldKeymap,
      ]),
      EditorView.updateListener.of((update) => {
        const tab = tabs.get(path);
        if (!tab) return;
        tab.state = update.state;
        if (update.docChanged) {
          renderTabs();
          announce(t("unsaved"), "warning");
          queueMicrotask(() => {
            if (editor === update.view) editor.dispatch(setDiagnostics(editor.state, []));
          });
        }
        if (update.selectionSet || update.docChanged) renderContextHelp(update.state, path);
      }),
      EditorView.lineWrapping,
      zelyraTheme,
      EditorView.contentAttributes.of({ "aria-label": `${path} im Zelyra-Webeditor` }),
    ],
  });
}

editorRoot.addEventListener("input", () => {
  if (!activePath || !editor) return;
  const tab = tabs.get(activePath);
  if (!tab) return;
  tab.state = editor.state;
  renderTabs();
  announce(t("unsaved"), "warning");
  scheduleLiveCheck(activePath);
});

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
  if (isDirty(tab, path) && !confirm(t("closeUnsaved", path))) return;
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
  if (!tab || !isDirty(tab, path)) return true;
  tab.state = tabState(path, tab);
  try {
    const result = await api("file", { method: "PUT", body: JSON.stringify({ path, content: tab.state.doc.toString(), baseHash: tab.hash }) });
    tab.saved = result.content;
    tab.hash = result.hash;
    symbolCache.delete(path);
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

async function renameActiveFile() {
  const path = activePath;
  if (!path || path === "main.zyl" || path === "zelyra.toml") return;
  const newPath = prompt(t("renamePrompt"), path);
  if (!newPath || newPath === path) return;
  if (!confirm(t("renameConfirm", path))) return;
  if (!(await savePath(path))) return;
  try {
    const tab = tabs.get(path);
    const result = await api("file-rename", { method: "POST", body: JSON.stringify({ path, newPath, baseHash: tab.hash }) });
    if (editor) { tab.state = editor.state; editor.destroy(); editor = null; }
    tabs.delete(path);
    tab.hash = result.hash;
    tabs.set(result.newPath, tab);
    activePath = result.newPath;
    await refreshTree();
    activate(result.newPath);
    announce(t("fileRenamed", result.newPath), "success");
  } catch (error) { announce(error.message, "error"); }
}

async function deleteActiveFile() {
  const path = activePath;
  if (!path || path === "main.zyl" || path === "zelyra.toml") return;
  if (!confirm(t("deleteConfirm", path))) return;
  if (!(await savePath(path))) return;
  try {
    const tab = tabs.get(path);
    await api("file", { method: "DELETE", body: JSON.stringify({ path, baseHash: tab.hash }) });
    closeTab(path);
    await refreshTree();
    announce(t("fileDeleted", path), "success");
  } catch (error) { announce(error.message, "error"); }
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
      row.type = "button";
      row.className = `tree-item${item.file ? " file" : " folder"}${item.path === activePath ? " selected" : ""}`;
      row.style.setProperty("--depth", depth);
      row.title = item.path;
      const icon = document.createElement("span"); icon.className = "tree-icon"; icon.setAttribute("aria-hidden", "true"); icon.textContent = item.file ? fileIcon(name) : "▸";
      const label = document.createElement("span"); label.className = "tree-label"; label.textContent = name;
      row.append(icon, label);
      let children = null;
      if (item.file) {
        if (item.path === activePath) row.setAttribute("aria-current", "page");
        row.addEventListener("click", () => openFile(item.path));
      } else {
        const childrenId = `tree-folder-${encodeURIComponent(item.path)}`;
        children = document.createElement("div");
        children.className = "tree-children";
        children.id = childrenId;
        row.setAttribute("aria-expanded", "true");
        row.setAttribute("aria-controls", childrenId);
        row.addEventListener("click", () => {
          const collapsed = !row.classList.contains("collapsed");
          row.classList.toggle("collapsed", collapsed);
          row.setAttribute("aria-expanded", String(!collapsed));
          children.hidden = collapsed;
        });
      }
      target.append(row);
      if (children) {
        row.after(children); draw(item, children, depth + 1);
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
    group.setAttribute("aria-label", `${table.name}, ${statusLabel(status)} · ${t("databaseOpenTable")}`);
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
  if (tab && isDirty(tab, path)) throw new Error(t("saveBeforePreview"));
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

function renderDiagnostics(items, success, options = {}) {
  diagnostics = items.map((item) => {
    const start = item.span?.start || {};
    return { ...item, from: item.editorSpan?.start ?? start.offset ?? 0, to: item.editorSpan?.end ?? item.editorSpan?.start ?? start.offset ?? 0 };
  });
  if (editor) editor.dispatch(setDiagnostics(editor.state, diagnostics.filter((item) => !item.file || item.file.endsWith(activePath)).map((item) => ({
    from: item.from, to: Math.max(item.from + 1, item.to), severity: item.severity || "error",
    message: `${item.code || ""} ${item.message || ""}`.trim(),
  }))));
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
  if (!options.silent) announce(success ? t("checkPassed") : t("problemsFound", items.length), success ? "success" : "error");
}

function scheduleLiveCheck(path) {
  if (!path?.endsWith(".zyl")) return;
  clearTimeout(liveCheckTimer);
  const sequence = ++liveCheckSequence;
  liveCheckTimer = setTimeout(async () => {
    if (sequence !== liveCheckSequence || activePath !== path) return;
    try {
      const source = editor?.state.doc.toString();
      if (typeof source !== "string") return;
      const result = await api("check", { method: "POST", body: JSON.stringify({ path, source }) });
      if (sequence !== liveCheckSequence || activePath !== path || editor?.state.doc.toString() !== source) return;
      renderDiagnostics(result.diagnostics || [], result.success, { silent: true });
    } catch {
      // Live checks are advisory; explicit project checks report request failures.
    }
  }, 700);
}

async function checkProject() {
  if (!(await saveAll())) return;
  announce(t("checkRunning"));
  try {
    const result = await api("check", { method: "POST", body: JSON.stringify({ path: activePath }) });
    renderDiagnostics(result.diagnostics || [], result.success);
  } catch (error) { announce(error.message, "error"); }
}

async function runProject() {
  if (!(await saveAll())) return;
  const dialog = $("run-output");
  const state = $("run-output-state");
  const output = $("run-output-content");
  state.textContent = t("runStarting");
  state.dataset.kind = "pending";
  output.textContent = "";
  if (!dialog.open) dialog.showModal();
  announce(t("runStarting"));
  try {
    const result = await api("run", { method: "POST", body: JSON.stringify({ path: "main.zyl" }) });
    const statusText = result.timedOut ? t("runTimeout") : result.success ? t("runSuccess") : t("runFailure");
    state.textContent = `${statusText}${result.exitCode === null ? "" : ` · exit ${result.exitCode}`} · ${result.durationMs} ms`;
    state.dataset.kind = result.success ? "success" : "error";
    const streams = [
      result.stdout && `${t("runStdout")}\n${result.stdout}`,
      result.stderr && `${t("runStderr")}\n${result.stderr}`,
    ].filter(Boolean).join("\n\n");
    output.textContent = streams || t("runNoOutput");
    output.textContent = `${output.textContent || t("runNoOutput")}\n\n${t("runInputUnavailable")}`;
    if (result.truncated) output.textContent += `\n${t("runOutputTruncated")}`;
    announce(statusText, result.success ? "success" : "error");
  } catch (error) {
    state.textContent = error.message;
    state.dataset.kind = "error";
    announce(error.message, "error");
  }
}

async function openProjectMap() {
  const dialog = $("project-map-dialog");
  const status = $("project-map-status");
  const graph = $("project-map-graph");
  if (!dialog.open) dialog.showModal();
  status.textContent = t("projectMapLoading");
  graph.replaceChildren();
  try {
    const map = await api("project-map");
    renderProjectMap(map);
    status.textContent = map.incomplete ? t("projectMapIncomplete") : "";
    if (!map.nodes?.length) status.textContent = t("projectMapEmpty");
  } catch (error) {
    status.textContent = error.message || t("projectMapUnavailable");
  }
}

function renderProjectMap(map) {
  const svg = $("project-map-graph");
  const ns = "http://www.w3.org/2000/svg";
  const columns = ["module", "data", "web", "logic"];
  const columnKinds = {
    module: new Set(["module"]),
    data: new Set(["table", "crud", "view"]),
    web: new Set(["api", "page"]),
    logic: new Set(["function"]),
  };
  const labels = { module: t("projectMapModules"), data: t("projectMapData"), web: t("projectMapWeb"), logic: t("projectMapLogic") };
  const grouped = Object.fromEntries(columns.map((column) => [column, []]));
  for (const node of map.nodes || []) {
    const column = columns.find((key) => columnKinds[key].has(node.kind));
    if (column) grouped[column].push(node);
  }
  for (const column of columns) grouped[column].sort((left, right) => left.label.localeCompare(right.label) || left.path.localeCompare(right.path));
  const width = 1080;
  const cardWidth = 224;
  const cardHeight = 50;
  const rowGap = 68;
  const xPositions = [16, 286, 556, 826];
  const height = Math.max(320, Math.max(...columns.map((column) => grouped[column].length), 1) * rowGap + 70);
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("width", String(width));
  svg.setAttribute("height", String(height));
  svg.replaceChildren();
  const defs = document.createElementNS(ns, "defs");
  const marker = document.createElementNS(ns, "marker");
  marker.setAttribute("id", "project-map-arrow");
  marker.setAttribute("viewBox", "0 0 8 8");
  marker.setAttribute("refX", "7"); marker.setAttribute("refY", "4");
  marker.setAttribute("markerWidth", "6"); marker.setAttribute("markerHeight", "6");
  marker.setAttribute("orient", "auto-start-reverse");
  const arrow = document.createElementNS(ns, "path");
  arrow.setAttribute("d", "M 0 0 L 8 4 L 0 8 z"); arrow.setAttribute("class", "project-map-arrow");
  marker.append(arrow); defs.append(marker); svg.append(defs);
  const positions = new Map();
  for (let columnIndex = 0; columnIndex < columns.length; columnIndex += 1) {
    const column = columns[columnIndex];
    const heading = document.createElementNS(ns, "text");
    heading.setAttribute("x", String(xPositions[columnIndex]));
    heading.setAttribute("y", "20");
    heading.setAttribute("class", "project-map-column-title");
    heading.textContent = labels[column];
    svg.append(heading);
    grouped[column].forEach((node, index) => {
      positions.set(node.id, { x: xPositions[columnIndex], y: 38 + index * rowGap });
    });
  }
  for (const edge of map.edges || []) {
    const from = positions.get(edge.from);
    const to = positions.get(edge.to);
    if (!from || !to) continue;
    const line = document.createElementNS(ns, "path");
    const startX = from.x + (from.x <= to.x ? cardWidth : 0);
    const endX = to.x + (from.x <= to.x ? 0 : cardWidth);
    const startY = from.y + cardHeight / 2;
    const endY = to.y + cardHeight / 2;
    const curve = Math.max(30, Math.abs(endX - startX) / 2);
    line.setAttribute("d", `M ${startX} ${startY} C ${startX + (from.x <= to.x ? curve : -curve)} ${startY}, ${endX - (from.x <= to.x ? curve : -curve)} ${endY}, ${endX} ${endY}`);
    line.setAttribute("class", `project-map-edge${edge.label === "imports" ? " import" : ""}`);
    line.setAttribute("marker-end", "url(#project-map-arrow)");
    const title = document.createElementNS(ns, "title");
    title.textContent = edge.label;
    line.append(title);
    svg.append(line);
    if (edge.label !== "declares") {
      const caption = document.createElementNS(ns, "text");
      caption.setAttribute("x", String((startX + endX) / 2));
      caption.setAttribute("y", String((startY + endY) / 2 - 4));
      caption.setAttribute("class", "project-map-edge-label");
      caption.textContent = edge.label;
      svg.append(caption);
    }
  }
  for (const column of columns) {
    for (const node of grouped[column]) {
      const position = positions.get(node.id);
      const group = document.createElementNS(ns, "g");
      group.setAttribute("class", `project-map-node kind-${node.kind}`);
      group.setAttribute("role", "button");
      group.setAttribute("tabindex", "0");
      group.setAttribute("aria-label", `${node.kind}: ${node.label} · ${node.path}:${node.line || 1} · ${t("projectMapOpenSource")}`);
      const rect = document.createElementNS(ns, "rect");
      rect.setAttribute("x", String(position.x)); rect.setAttribute("y", String(position.y));
      rect.setAttribute("width", String(cardWidth)); rect.setAttribute("height", String(cardHeight));
      const name = document.createElementNS(ns, "text");
      name.setAttribute("x", String(position.x + 10)); name.setAttribute("y", String(position.y + 20)); name.setAttribute("class", "project-map-node-name");
      name.textContent = `${node.kind} · ${node.label}`.slice(0, 32);
      const file = document.createElementNS(ns, "text");
      file.setAttribute("x", String(position.x + 10)); file.setAttribute("y", String(position.y + 38)); file.setAttribute("class", "project-map-node-file");
      file.textContent = `${node.path}:${node.line || 1}`.slice(0, 34);
      const title = document.createElementNS(ns, "title");
      title.textContent = `${node.kind}: ${node.label} · ${node.path}:${node.line || 1}`;
      group.append(rect, name, file, title);
      const openNode = () => {
        void openFile(node.path).then(() => {
          if (activePath !== node.path || !editor) return;
          const position = Math.max(0, Math.min(node.position || 0, editor.state.doc.length));
          editor.dispatch({ selection: { anchor: position }, scrollIntoView: true });
          editor.focus();
          $("project-map-dialog").close();
        });
      };
      group.addEventListener("click", openNode);
      group.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") { event.preventDefault(); openNode(); }
      });
      svg.append(group);
    }
  }
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
$("complete-button").addEventListener("click", () => { if (editor) { editor.focus(); startCompletion(editor); } });
$("rename-file-button").addEventListener("click", renameActiveFile);
$("delete-file-button").addEventListener("click", deleteActiveFile);
$("save-all-button").addEventListener("click", saveAll);
$("check-button").addEventListener("click", checkProject);
$("run-button").addEventListener("click", runProject);
$("format-button").addEventListener("click", formatActive);
$("database-button").addEventListener("click", openDatabaseWorkbench);
$("assistant-toggle").addEventListener("click", () => toggleAssistant());
$("assistant-help-tab").addEventListener("click", () => showAssistantTab("help"));
$("assistant-properties-tab").addEventListener("click", () => showAssistantTab("properties"));
$("assistant-help-tab").addEventListener("keydown", (event) => {
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    event.preventDefault();
    showAssistantTab("properties");
    $("assistant-properties-tab").focus();
  }
});
$("assistant-properties-tab").addEventListener("keydown", (event) => {
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    event.preventDefault();
    showAssistantTab("help");
    $("assistant-help-tab").focus();
  }
});
$("assistant-format").addEventListener("click", formatActive);
$("assistant-check").addEventListener("click", checkProject);
$("task-finder-button").addEventListener("click", openTaskFinder);
$("task-finder-close").addEventListener("click", () => $("task-finder").close());
$("task-finder").addEventListener("close", () => $("task-finder-input").setAttribute("aria-expanded", "false"));
$("project-search-button").addEventListener("click", openProjectSearch);
$("project-search-close").addEventListener("click", () => $("project-search").close());
$("run-output-close").addEventListener("click", () => $("run-output").close());
$("project-map-button").addEventListener("click", openProjectMap);
$("project-map-close").addEventListener("click", () => $("project-map-dialog").close());
$("project-search-input").addEventListener("keydown", (event) => {
  if (event.key === "Enter") { event.preventDefault(); void performProjectSearch(); }
});
$("task-finder-input").addEventListener("input", () => { selectedTaskIndex = 0; renderTaskFinder(); });
$("task-finder-input").addEventListener("keydown", (event) => {
  const rows = [...$("task-finder-results").querySelectorAll(".task-finder-item")];
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    selectedTaskIndex = rows.length ? (selectedTaskIndex + (event.key === "ArrowDown" ? 1 : -1) + rows.length) % rows.length : -1;
    rows.forEach((row, index) => row.setAttribute("aria-selected", String(index === selectedTaskIndex)));
    if (selectedTaskIndex >= 0) $("task-finder-input").setAttribute("aria-activedescendant", rows[selectedTaskIndex].id);
    rows[selectedTaskIndex]?.scrollIntoView({ block: "nearest" });
  } else if (event.key === "Enter" && rows[selectedTaskIndex]) {
    event.preventDefault(); rows[selectedTaskIndex].click();
  }
});
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
$("sidebar-toggle").addEventListener("click", (event) => {
  const hidden = document.body.classList.toggle("sidebar-hidden");
  event.currentTarget.setAttribute("aria-expanded", String(!hidden));
});
$("problems-toggle").addEventListener("click", () => { $("problems-panel").hidden = !$("problems-panel").hidden; });
$("problems-close").addEventListener("click", () => { $("problems-panel").hidden = true; });
$("open-search").addEventListener("click", findInDocument);
window.addEventListener("beforeunload", (event) => {
  if (dirtyCount()) { event.preventDefault(); event.returnValue = ""; }
});
window.addEventListener("keydown", (event) => {
  if (!event.defaultPrevented && (event.ctrlKey || event.metaKey) && event.code === "Space" && editor) {
    event.preventDefault();
    startCompletion(editor);
  }
  if (event.key === "Escape" && $("task-finder").open) { event.preventDefault(); $("task-finder").close(); }
  if (event.key === "Escape" && $("project-search").open) { event.preventDefault(); $("project-search").close(); }
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "p") { event.preventDefault(); openTaskFinder(); }
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "f") { event.preventDefault(); openProjectSearch(); }
  if (event.key === "F1") { event.preventDefault(); toggleAssistant("help"); }
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
