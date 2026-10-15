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
import { Compartment, EditorState, StateEffect } from "@codemirror/state";
import { tags } from "@lezer/highlight";
import {
  zelyraUiAppearanceValues,
  zelyraUiComponentMetadata,
  zelyraUiDefaultAppearance,
} from "./zelyra-ui-metadata.js";
import {
  crosshairCursor,
  drawSelection,
  EditorView,
  GutterMarker,
  gutter,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";

const root = document.documentElement;
const storedTheme = localStorage.getItem("zelyra-editor-theme");
root.dataset.theme = storedTheme === "light" ? "light" : "dark";
const storedFontSize = localStorage.getItem("zelyra-editor-font-size");
if (storedFontSize && ["medium", "large"].includes(storedFontSize)) {
  root.dataset.fontSize = storedFontSize;
}
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
let editorLogRecords = [];
let editor = null;
let diagnostics = [];
let verificationResults = [];
const verificationRefresh = StateEffect.define();

class VerificationMarker extends GutterMarker {
  constructor(status, explanation) {
    super();
    this.status = status;
    this.explanation = explanation;
  }
  toDOM() {
    const marker = document.createElement("span");
    marker.className = `verify-marker ${this.status.toLowerCase()}`;
    marker.textContent = this.status === "PROVEN" ? "◆" : this.status === "FAILED" ? "✖" : "●";
    marker.title = this.explanation;
    marker.setAttribute("role", "button");
    marker.setAttribute("tabindex", "0");
    marker.setAttribute("aria-label", this.explanation);
    marker.addEventListener("click", () => announce(this.explanation, this.status === "FAILED" ? "error" : ""));
    marker.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        announce(this.explanation, this.status === "FAILED" ? "error" : "");
      }
    });
    return marker;
  }
}

function setVerificationResults(results) {
  verificationResults = results;
  if (editor) editor.dispatch({ effects: verificationRefresh.of(null) });
}

const verificationGutter = gutter({
  class: "verify-gutter",
  lineMarker(view, line) {
    const number = view.state.doc.lineAt(line.from).number;
    const entries = verificationResults.filter((item) => item.file === activePath && item.span?.start?.line === number);
    if (!entries.length) return null;
    const priority = { FAILED: 0, RUNTIME_CHECK: 1, UNPROVEN: 2, PROVEN: 3 };
    entries.sort((left, right) => priority[left.status] - priority[right.status]);
    const explanation = entries.map((item) => `${item.status} ${item.code}: ${item.message}`).join("\n");
    return new VerificationMarker(entries[0].status, explanation);
  },
  lineMarkerChange(update) {
    return update.transactions.some((transaction) => transaction.effects.some((effect) => effect.is(verificationRefresh)));
  },
});
let liveCheckTimer = null;
let liveCheckSequence = 0;
let pipelineMapCache = null;
const eventClientId = crypto.randomUUID();
let eventController = null;
let eventReconnectTimer = null;
let eventReconnectDelay = 1500;

function onStudioEvent(message) {
  if (message?.schema !== 1 || typeof message.kind !== "string") return;
  if (message.kind === "workspace.changed") {
    void refreshTree();
    const changedPath = message.document?.path || "";
    if (!changedPath || changedPath === activePath) setVerificationResults([]);
    pipelineMapCache = null;
    renderPipelineBreadcrumbs();
    if (message.payload?.operation?.startsWith("external-")) {
      void refreshExternallyChangedTabs(changedPath).then(() => {
        if (activePath?.endsWith(".zyl")) scheduleLiveCheck(activePath);
        scheduleCanvasRefresh();
        if ((!changedPath || changedPath === "zelyra.theme.css") && !$("canvas-pane").hidden) void loadThemeColors();
      });
    } else {
      if (activePath?.endsWith(".zyl")) scheduleLiveCheck(activePath);
      scheduleCanvasRefresh();
    }
    return;
  }
  if (message.kind === "analysis.invalidated") {
    if (message.document?.path === activePath) setVerificationResults([]);
    return;
  }
  if (message.kind === "preview.changed") {
    if (message.payload?.running) announce(t("runWebReady"), "success");
    return;
  }
  const document = message.document;
  if (document?.clientId !== eventClientId || document.path !== activePath
      || document.version !== liveCheckSequence || !editor) return;
  if (message.kind === "diagnostics") {
    renderDiagnostics(message.payload?.diagnostics || [], Boolean(message.payload?.success), { silent: true });
  } else if (message.kind === "verification") {
    setVerificationResults(message.payload?.success ? message.payload.results || [] : []);
  }
}

async function connectStudioEvents() {
  if (!token || eventController) return;
  eventController = new AbortController();
  const current = eventController;
  try {
    const response = await fetch("/__zelyra/editor/api/events", {
      headers: { "X-Zelyra-Editor-Token": token },
      cache: "no-store", credentials: "omit", redirect: "error", signal: current.signal,
    });
    if (!response.ok || !response.body) throw new Error(`Studio events: HTTP ${response.status}`);
    eventReconnectDelay = 1500;
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true }).replaceAll("\r\n", "\n");
      if (buffer.length > 300_000) throw new Error("Studio event exceeds the client limit");
      let end;
      while ((end = buffer.indexOf("\n\n")) >= 0) {
        const frame = buffer.slice(0, end);
        buffer = buffer.slice(end + 2);
        const data = frame.split("\n").filter((line) => line.startsWith("data: "))
          .map((line) => line.slice(6)).join("\n");
        if (data) {
          try { onStudioEvent(JSON.parse(data)); } catch { /* malformed advisory event */ }
        }
      }
    }
  } catch {
    // The request APIs remain usable while the advisory stream reconnects.
  } finally {
    if (eventController === current) eventController = null;
    if (!current.signal.aborted) {
      eventReconnectTimer = setTimeout(connectStudioEvents, eventReconnectDelay);
      eventReconnectDelay = Math.min(eventReconnectDelay * 2, 10_000);
    }
  }
}
const symbolCache = new Map();
const symbolRequests = new Map();
let treeFiles = [];
let activeFilter = "";
let projectSearchMode = "text";
let semanticReferenceQuery = null;
let databaseMap = null;
let databaseTables = [];
let selectedDatabaseTable = null;
let databaseFilter = "";
let databaseRecordOffset = 0;
let databaseRecordOffsets = [0];
let databaseRecordRequest = 0;
let databaseRecordBusy = false;
let chartPreviewState = null;
let presentationCharts = [];
let schemaColumnPreview = null;
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
    taskRun: "Programm oder Web-App starten", taskRunHelp: "Konsolenprogramme mit Zelyras Laufzeit ausführen oder Webanwendungen lokal starten; konfigurierte Effekte sind möglich, interaktive Eingabe fehlt",
    projectMap: "Projektkarte", projectMapTitle: "Projektbeziehungen", projectMapOpenSource: "Quelltext öffnen", projectMapLoading: "Compiler-Projektmodell wird geladen …", projectMapUnavailable: "Projektkarte konnte nicht erstellt werden.", projectMapIncomplete: "Die Karte ist unvollständig. Nicht auflösbare oder begrenzte Beziehungen werden nicht geraten.", projectMapIssues: { "view.unresolved": "Ansicht nicht gefunden", "view.ambiguous": "Ansichtsname mehrfach vorhanden", "crud.table_unresolved": "CRUD-Tabelle nicht eindeutig", "api.handler_unresolved": "API-Funktion nicht eindeutig", "import.unresolved": "Importziel nicht gefunden", "edge.target_missing": "Beziehungsziel fehlt", "limit.nodes": "Knotenlimit erreicht", "limit.edges": "Beziehungslimit erreicht" }, projectMapEmpty: "Noch keine unterstützten Beziehungen im Projekt.", projectMapModules: "Module", projectMapData: "Daten und Ansichten", projectMapWeb: "Routen und APIs", projectMapLogic: "Funktionen",
    pipelineBreadcrumbs: "Semantische Projektbeziehungen", projectMapNullability: "NULL-Radar (Compiler E-SQL-004)", projectMapIssuesExtra: { "form.table_unresolved": "Formular-Tabelle nicht eindeutig", "struct.unresolved": "Ergebnistyp nicht gefunden", "struct.ambiguous": "Ergebnistyp mehrfach vorhanden", "sql.access_unresolved": "SQL-Tabellenbezug nicht eindeutig" },
    pdfPreview: "PDF-Vorschau", pdfPreviewTitle: "PDF-Vorschau mit echten PDF-Bytes", pdfView: "Deklarierte View", pdfExampleData: "Beispieldaten als JSON-Objekt", pdfRender: "PDF erzeugen", pdfDownload: "PDF herunterladen", pdfNoViews: "Im Projekt ist keine View deklariert.", pdfReady: "PDF erzeugt ({0} Byte).", pdfLoading: "Views werden geladen …", pdfRendering: "PDF wird lokal erzeugt …", pdfInvalidData: "Bitte ein gültiges JSON-Objekt als Beispieldaten eingeben.", pdfError: "PDF-Vorschau fehlgeschlagen: {0}",
    outboxButton: "Outbox", outboxTitle: "Mail-Outbox", outboxCaption: "Metadaten der MariaDB-Outbox ohne Nachrichteninhalt", outboxEvent: "Ereignis", outboxStatus: "Status", outboxRecipient: "Empfänger", outboxAttachment: "Anhang", outboxCreated: "Erstellt", outboxAvailable: "Verfügbar", outboxError: "Fehler", outboxAction: "Aktion", previous: "Zurück", next: "Weiter", outboxLoading: "Outbox wird geladen …", outboxEmpty: "Keine Ereignisse auf dieser Seite.", outboxPage: "{0}–{1} von {2}", outboxRequeue: "Erneut einreihen", outboxRequeueConfirm: "Das erschöpfte Ereignis {0} für einen späteren Worker-Lauf freigeben? Es wird jetzt keine E-Mail versendet.", outboxRequeued: "Ereignis erneut eingereiht. Ein Versand erfolgt erst durch einen ausdrücklich gestarteten Worker.", outboxLoadError: "Outbox konnte nicht geladen werden: {0}", outboxRequeueError: "Erneutes Einreihen fehlgeschlagen: {0}",
    outboxAttempts: "Versuche: {0}", outboxLeaseUntil: "Sperre bis: {0}", outboxDeliveredAt: "Zugestellt: {0}", outboxExhaustedAt: "Erschöpft: {0}", outboxNoDatabase: "Für die Outbox ist keine MariaDB-Verbindung konfiguriert. Trage DATABASE_URL in .env oder der Umgebung ein.",
    capabilityButton: "Rechte", capabilityTitle: "Capability- und Sicherheitsmatrix", capabilityDeclarations: "Deklarierte Rechte", capabilityUses: "Tatsächlich verwendete Operationen", capabilityPaths: "Dateipfade", capabilityUnused: "Freigaben ohne erkannten Funktionsbedarf", capabilityUnusedCaveat: "Nur analysierte Funktionen; generierte Routen und dynamische Aufrufe können weitere Rechte benötigen.", capabilityLoading: "Compiler und Projektkonfiguration werden analysiert …", capabilityEmpty: "Keine Einträge in der aktuellen Datei.", capabilityGrant: "Freigabe prüfen", capabilityGrantReview: "Änderung an zelyra.toml prüfen", capabilityGrantApply: "Freigabe übernehmen", capabilityGrantPreview: "Nur die angezeigte Konfigurationsänderung wird nach deiner Bestätigung gespeichert.", capabilityGranted: "Freigabe gespeichert. Die Runtime prüft sie beim nächsten Lauf erneut.", capabilityUnrestricted: "Keine Projektkonfiguration: Runtime-Capabilities sind nicht durch zelyra.toml eingeschränkt.", capabilityIncomplete: "Die Quellanalyse ist unvollständig; dieser Befund kann Einträge auslassen.", capabilityError: "Capability-Analyse fehlgeschlagen: {0}", capabilityPathUnknown: "Dynamische Pfade sind statisch nicht entscheidbar.", capabilityNotGranted: "nicht freigegeben", capabilityDynamic: "dynamisch",
    externalReloaded: "Extern geänderte Datei neu geladen: {0}", externalDirty: "{0} wurde außerhalb von Studio geändert. Ungespeicherte Änderungen bleiben erhalten; vor dem Speichern prüfen.", externalMissing: "{0} wurde außerhalb von Studio entfernt. Der geöffnete Puffer bleibt erhalten.",
    externalReloadFailed: "{0} konnte nach einer äußeren Änderung nicht neu geladen werden: {1}",
    canvasButton: "Canvas", canvasTitle: "Statische Zelyra-Vorschau", canvasTarget: "Seite oder View", canvasSample: "Beispieldaten (JSON)", canvasRender: "Vorschau aktualisieren", canvasSource: "Zur Deklaration", canvasFrameTitle: "Statische Seitenvorschau", canvasLoading: "Compiler und Renderer erzeugen die Vorschau …", canvasEmpty: "Im Projekt ist keine Seite oder View deklariert.", canvasError: "Vorschau nicht verfügbar: {0}", canvasSampleError: "Beispieldaten müssen ein JSON-Objekt sein.", canvasReady: "Statisch gerendert. Projektfunktionen und Skripte wurden nicht ausgeführt.", canvasIncomplete: "Die Projektquelle ist nicht vollständig kompilierbar.",
    canvasThemeToken: "Theme-Farbe", canvasThemeColor: "Farbwert", canvasThemeApply: "Im Quelltext ändern", canvasThemeMissing: "Für Farbbearbeitung zelyra.theme.css im Projekt anlegen.", canvasThemeNoHex: "Keine unterstützte --z-color-*-Farbe mit hexadezimalem Wert gefunden.", canvasThemeChanged: "Theme-Farbe im ungespeicherten Quelltext geändert. Speichern übernimmt sie ins Projekt.", canvasThemeStale: "Theme-Datei wurde verändert. Farbe erneut auswählen.", canvasThemeError: "Theme-Bearbeitung fehlgeschlagen: {0}",
    runButton: "Starten", runButtonTitle: "main.zyl ausführen oder Webanwendung starten",
    runOutputTitle: "Programmausgabe", runStarting: "Anwendung wird vorbereitet und gestartet …", runSuccess: "Programm erfolgreich beendet", runFailure: "Programm mit Fehler beendet", runTimeout: "Ausführung nach 15 Sekunden beendet", runOutputTruncated: "Die Ausgabe ist auf 64 KiB je Stream begrenzt.", runNoOutput: "Keine Ausgabe.", runStdout: "AUSGABE", runStderr: "FEHLER", runInputUnavailable: "Studio stellt dem Programm keine interaktive Eingabe bereit.", runWebReady: "Webanwendung läuft lokal:", runWebOpen: "Anwendung im Browser öffnen", runWebStop: "Vorschau beenden", runWebStopped: "Lokale Vorschau wurde beendet.",
    runtimeBudgetButton: "Budget", runtimeBudgetTitle: "Laufzeitbudget und Rekursion", runtimeBudgetEnable: "Schrittbudget für Konsolenprogramme aktivieren", runtimeBudgetRange: "Schrittbudget (Regler: 1.000 bis 1.000.000)", runtimeBudgetNumber: "Maximale Schritte (1 bis 100.000.000)", runtimeBudgetWebHint: "Das Schrittbudget gilt für Konsolenprogramme. Eine Webvorschau verwendet weiterhin ihre eigenen Laufzeitgrenzen.", runtimeRecursionTitle: "Mögliche Rekursion", runtimeRecursionNone: "Keine direkte oder gegenseitige Rekursion im analysierten Funktionsgraphen gefunden.", runtimeRecursionIncomplete: "Der Funktionsgraph ist zu groß; die Rekursionsanalyse ist unvollständig.", runtimeRecursionDirect: "Direkte Rekursion", runtimeRecursionMutual: "Gegenseitige Rekursion", runtimeStackDepth: "Maximale Stacktiefe laut Projekt: {0} ({1})", runtimeStackConfigured: "konfiguriert", runtimeStackDefault: "Standard", runtimeProfileError: "Laufzeitprofil konnte nicht gelesen werden: {0}", runtimeBudgetInvalid: "Bitte eine ganze Schrittzahl von 1 bis 100.000.000 eingeben.", runtimeBudgetExceeded: "Schrittbudget überschritten", runCancel: "Ausführung abbrechen", runCancelRequested: "Abbruch angefordert …", runCancelled: "Ausführung abgebrochen", runtimeProfileInvalid: "Der ungespeicherte Quelltext ist nicht vollständig analysierbar.",
    logs: "Logs", logsTitle: "Zelyra-Logs", logsLoading: "Logs werden geladen …", logsUnavailable: "Logs konnten nicht geladen werden.", logsEmpty: "Noch keine Programmausgabe protokolliert.", logsLevel: "Stufe", logsAll: "Alle", logsSearch: "Text filtern", logsRetention: "Lokal gespeichert; begrenzte Rotation", logsCount: "{0} Einträge angezeigt",
    taskHelp: "Kontexthilfe öffnen", taskHelpHelp: "Hilfe zum Begriff am Cursor anzeigen", taskProperties: "Dateieigenschaften öffnen",
    taskPropertiesHelp: "Informationen und Aktionen zur aktuellen Datei anzeigen", taskSearch: "Text suchen", taskSearchHelp: "Im aktuellen Dokument suchen",
    taskGoToDefinition: "Zur Definition springen", taskGoToDefinitionHelp: "Zur Funktion oder zum Typ am Cursor springen (F12)",
    taskFindReferences: "Bezeichnerverwendungen finden", taskFindReferencesHelp: "Exakte Zelyra-Bezeichner im Projekt finden (Umschalt+F12)",
    definitionNotFound: "Keine passende Definition im Projekt gefunden.", definitionSaveFirst: "Speichere die Datei vor dem Sprung, damit die Quellposition stimmt.", definitionOpened: "Definition geöffnet: {0}", analysisOverlayLimit: "Die ungespeicherten Zelyra-Dateien überschreiten die Analysegrenze (15 Dateien oder 1 MiB).",
    semanticRenameTitle: "Funktion umbenennen", semanticRenameName: "Neuer Funktionsname", semanticRenamePreview: "Änderungsvorschau", semanticRenameApply: "Änderungen in Puffern übernehmen", semanticRenameReady: "{0} Dateien, {1} Fundstellen geprüft. Vor dem Speichern einzeln rückgängig machbar.", semanticRenameApplied: "{0} Dateien geändert. Prüfe und speichere die Puffer.", semanticRenameStale: "Eine Quelldatei hat sich seit der Vorschau geändert. Bitte erneut prüfen.",
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
    renameConfirm: "Datei {0} nach {1} umbenennen? {2}", renameImports: "Importe werden in folgenden Dateien aktualisiert:\n{0}", renameNoImports: "Es sind keine Importverweise betroffen.", deleteConfirm: "Datei {0} endgültig löschen?",
    fileRenamed: "Datei umbenannt: {0}", fileDeleted: "Datei gelöscht: {0}",
    switchLanguage: "Sprache wechseln", switchFontSize: "Schriftgröße anpassen", fontSizeDefaultAction: "Standard-Schriftgröße (100%)", fontSizeDefaultHelp: "Schriftgröße der Elemente und des Editors auf Standard zurücksetzen", fontSizeMediumAction: "Mittlere Schriftgröße (+15%)", fontSizeMediumHelp: "Schriftgröße um 15% vergrößern", fontSizeLargeAction: "Große Schriftgröße (+30%)", fontSizeLargeHelp: "Schriftgröße um 30% vergrößern", themeLightAction: "Helles Farbschema aktivieren", themeDarkAction: "Dunkles Farbschema aktivieren",
    format: "Formatieren", check: "Projekt prüfen", welcome: "Willkommen in Zelyra Studio",
    welcomeHint: "Öffne links eine Datei und entwickle direkt in deinem Projekt.", problems: "Probleme",
    noProblems: "Keine Probleme gefunden. Dein Projekt ist bereit.", close: "Panel schließen", loadingFiles: "Dateien werden geladen",
    toggleExplorer: "Dateiübersicht umschalten", search: "Suchen", handbook: "Handbuch", projectFiles: "Projektdateien",
    openFile: "Datei öffnen", complete: "Vervollständigen", completeHelp: "Codevorschläge anzeigen (Strg+Leertaste)", ready: "Bereit", localMode: "Lokaler Modus",
    noDocument: "Kein Dokument geöffnet", opened: "{0} geöffnet", unsaved: "Ungespeicherte Änderungen",
    saved: "{0} gespeichert", savedAll: "{0} Dateien gemeinsam gespeichert", checkRunning: "Projekt wird geprüft …", checkPassed: "Projektprüfung erfolgreich",
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
    recordPreviewEyebrow: "DATEN VORSCHAU", recordPreviewTitle: "Datensätze ansehen",
    recordPreviewHint: "Nur-Leseansicht · bis zu 25 Zeilen pro Seite · Laden erst nach deiner Auswahl.",
    recordPreviewLoad: "Datensätze laden", recordPreviewPrevious: "Zurück", recordPreviewNext: "Weiter",
    recordPreviewIdle: "Wähle eine Tabelle und lade die Vorschau bewusst.", recordPreviewLoading: "Lese Datensätze …",
    recordPreviewPage: "Zeilen {0}–{1}", recordPreviewEmpty: "Keine Datensätze auf dieser Seite.",
    recordPreviewIncomplete: "Die Vorschau ist begrenzt. Lange Werte oder zusätzliche Spalten wurden gekürzt.",
    recordPreviewFailed: "Datensätze konnten nicht geladen werden.",
    chartEyebrow: "GESCHÄFTSDIAGRAMM", chartTitle: "Daten verständlich zeigen", chartIntro: "Wähle eine Gruppierung und eine Kennzahl. Studio fragt nur die ausgewählte Tabelle ab.",
    chartName: "Diagrammtitel", chartGroup: "Gruppieren nach", chartAggregation: "Kennzahl", chartCount: "Anzahl Datensätze", chartSum: "Summe", chartAvg: "Durchschnitt", chartMin: "Kleinster Wert", chartMax: "Größter Wert", chartMeasure: "Wert aus Feld", chartKind: "Darstellung", chartBars: "Säulen", chartLine: "Linie", chartFilterColumn: "Textfilterfeld", chartFilterValue: "Exakter Filterwert", chartNoFilter: "Kein Filter", chartPeriodColumn: "Datumsfeld", chartPeriodFrom: "Von einschließlich", chartPeriodTo: "Bis einschließlich", chartPreview: "Diagramm ansehen", chartSave: "Diagramm speichern", chartPreviewRunning: "Berechne sichere Vorschau …", chartPreviewReady: "Vorschau bereit · höchstens 20 Gruppen", chartPreviewFailed: "Diagramm konnte nicht berechnet werden.", chartSaved: "Diagrammkonfiguration gespeichert: {0}", chartSaveFailed: "Diagramm konnte nicht gespeichert werden.", chartSaveFirst: "Zeige zuerst eine aktuelle Diagrammvorschau.", chartTruncated: "Die Vorschau zeigt höchstens 20 Gruppen.", chartEmpty: "Keine gruppierten Daten gefunden.", chartGroupLabel: "Gruppe", chartValueLabel: "Wert", chartAccessible: "{0} gruppiert nach {1}", chartDefaultTitle: "Auswertung {0}", chartInvalidNumber: "Nichtnumerischer Kennzahlwert",
    presentationEyebrow: "PRÄSENTATION", presentationTitle: "Geschäftsergebnisse teilen", presentationIntro: "Füge geprüfte Diagramme hinzu und öffne eine druckfertige, lokal erzeugte PDF-Vorschau.", presentationName: "Titel", presentationNote: "Einleitung", presentationEmpty: "Noch keine Diagramme", presentationCount: "{0} Diagramm(e) · {1}", presentationAdd: "Zur Präsentation hinzufügen", presentationRemove: "Entfernen", presentationSave: "Präsentation speichern", presentationSaved: "Präsentation gespeichert: {0}", presentationLoaded: "Präsentation geladen: {0}", presentationSaveFailed: "Präsentation konnte nicht gespeichert werden: {0}", presentationPdf: "Druckansicht / PDF", presentationNeedCharts: "Füge mindestens ein Diagramm hinzu.", presentationPrintFailed: "Die Präsentationsansicht konnte nicht geöffnet werden.", presentationLoadFailed: "Ein Diagramm konnte für die Präsentation nicht geladen werden.", presentationKicker: "ZELYRA GESCHÄFTSAUSWERTUNG", presentationFooter: "Lokal erstellt mit Zelyra Studio", presentationNumber: "Folie {0} von {1}",
    savedChart: "Gespeichertes Diagramm", savedChartAdd: "Laden und hinzufügen", savedPresentation: "Gespeicherte Präsentation", savedPresentationLoad: "Präsentation öffnen", reportFilesEmpty: "Keine gespeicherten Auswertungen gefunden.", chartInvalidConfig: "Die gespeicherte Diagrammdatei ist ungültig oder nicht unterstützt.", chartAdded: "Gespeichertes Diagramm zur Präsentation hinzugefügt.", reportFileConflict: "Am Ziel liegt bereits eine Datei mit anderem Inhaltstyp.", presentationInvalidConfig: "Die gespeicherte Präsentation ist ungültig oder nicht unterstützt.",
    createDatabase: "MariaDB-Datenbank anlegen", databaseTargetLoading: "Prüfe Projektziel …",
    databaseTargetMissing: "Kein unterstütztes MariaDB-Ziel mit Verbindung konfiguriert.",
    databaseTargetReady: "MariaDB-Ziel {0} · Verbindung konfiguriert",
    databaseCreateConfirm: "Die MariaDB-Datenbank {0} anlegen oder sicherstellen, dass sie existiert? Die Zugangsdaten werden nur serverseitig verwendet.",
    databaseCreated: "Datenbank vorhanden. Prüfe den Schema-Plan und wende ihn separat an.",
    tableNotDeclared: "Diese Live-Tabelle ist noch nicht in Zelyra deklariert. Lege zuerst eine geprüfte Tabellendeklaration an.",
    existingCrud: "Für diese Tabelle existiert bereits eine CRUD-Ressource.",
    previewRunning: "Prüfe erzeugten Code …", previewPassed: "Compilerprüfung bestanden · Änderung noch nicht übernommen",
    previewFailed: "Compilerprüfung fehlgeschlagen", previewRequired: "Erstelle zuerst eine gültige Vorschau.",
    applySuccess: "CRUD-Änderung übernommen", selectOneField: "Wähle mindestens ein Feld.",
    schemaColumnTitle: "Tabellenspalten bearbeiten", schemaColumnHint: "Erzeugt und prüft eine Änderung im Zelyra-Quelltext. Die Datenbank wird dabei nicht geändert.",
    schemaColumnOperation: "Aktion", schemaColumnAdd: "Hinzufügen", schemaColumnEdit: "Bearbeiten", schemaColumnExisting: "Vorhandenes Feld",
    schemaColumnName: "Feldname", schemaColumnType: "Datentyp", schemaColumnRequired: "Pflichtfeld",
    schemaColumnPreview: "Änderung prüfen", schemaColumnApply: "Quelltext übernehmen",
    schemaColumnRunning: "Prüfe Tabellendeklaration …", schemaColumnPassed: "Compilerprüfung bestanden · Datenbank unverändert",
    schemaColumnFailed: "Compilerprüfung fehlgeschlagen", schemaColumnConfirm: "Feld {0} im Projektquelltext ändern? Die Datenbank bleibt unverändert. Prüfe danach den Schema-Plan.",
    schemaColumnApplied: "Quelltext geändert. Prüfe `zelyra db plan main.zyl`, bevor du eine Datenbankänderung anwendest.",
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
    taskRun: "Run program or web app", taskRunHelp: "Run console programs with Zelyra or start web applications locally; configured effects are possible, and interactive input is unavailable",
    projectMap: "Project map", projectMapTitle: "Project relationships", projectMapOpenSource: "Open source location", projectMapLoading: "Loading the compiler project model …", projectMapUnavailable: "Could not create the project map.", projectMapIncomplete: "The map is incomplete. Unresolved or limited relationships are not guessed.", projectMapIssues: { "view.unresolved": "View not found", "view.ambiguous": "View name is ambiguous", "crud.table_unresolved": "CRUD table is not unique", "api.handler_unresolved": "API function is not unique", "import.unresolved": "Import target not found", "edge.target_missing": "Relationship target is missing", "limit.nodes": "Node limit reached", "limit.edges": "Relationship limit reached" }, projectMapEmpty: "No supported relationships in this project yet.", projectMapModules: "Modules", projectMapData: "Data and views", projectMapWeb: "Routes and APIs", projectMapLogic: "Functions",
    pipelineBreadcrumbs: "Semantic project relationships", projectMapNullability: "NULL radar (compiler E-SQL-004)", projectMapIssuesExtra: { "form.table_unresolved": "Form table is not unique", "struct.unresolved": "Result type not found", "struct.ambiguous": "Result type is ambiguous", "sql.access_unresolved": "SQL table access is unresolved" },
    pdfPreview: "PDF preview", pdfPreviewTitle: "PDF preview with real PDF bytes", pdfView: "Declared view", pdfExampleData: "Example data as a JSON object", pdfRender: "Render PDF", pdfDownload: "Download PDF", pdfNoViews: "This project declares no view.", pdfReady: "PDF rendered ({0} bytes).", pdfLoading: "Loading views …", pdfRendering: "Rendering PDF locally …", pdfInvalidData: "Enter a valid JSON object for example data.", pdfError: "PDF preview failed: {0}",
    outboxButton: "Outbox", outboxTitle: "Mail outbox", outboxCaption: "MariaDB outbox metadata without message bodies", outboxEvent: "Event", outboxStatus: "Status", outboxRecipient: "Recipient", outboxAttachment: "Attachment", outboxCreated: "Created", outboxAvailable: "Available", outboxError: "Error", outboxAction: "Action", previous: "Previous", next: "Next", outboxLoading: "Loading outbox …", outboxEmpty: "No events on this page.", outboxPage: "{0}–{1} of {2}", outboxRequeue: "Requeue", outboxRequeueConfirm: "Release exhausted event {0} for a later worker run? No email will be sent now.", outboxRequeued: "Event requeued. Delivery requires a separately started worker.", outboxLoadError: "Could not load outbox: {0}", outboxRequeueError: "Requeue failed: {0}",
    outboxAttempts: "Attempts: {0}", outboxLeaseUntil: "Lease until: {0}", outboxDeliveredAt: "Delivered: {0}", outboxExhaustedAt: "Exhausted: {0}", outboxNoDatabase: "No MariaDB connection is configured for the outbox. Set DATABASE_URL in .env or the environment.",
    capabilityButton: "Permissions", capabilityTitle: "Capability and security matrix", capabilityDeclarations: "Declared capabilities", capabilityUses: "Operations actually used", capabilityPaths: "File paths", capabilityUnused: "Grants without a detected function use", capabilityUnusedCaveat: "Analyzed functions only; generated routes and dynamic calls may require more permissions.", capabilityLoading: "Analyzing compiler and project configuration …", capabilityEmpty: "No entries in the current file.", capabilityGrant: "Review grant", capabilityGrantReview: "Review change to zelyra.toml", capabilityGrantApply: "Apply grant", capabilityGrantPreview: "Only the displayed configuration change will be saved after your confirmation.", capabilityGranted: "Grant saved. The runtime checks it again on the next run.", capabilityUnrestricted: "No project configuration: runtime capabilities are not restricted by zelyra.toml.", capabilityIncomplete: "Source analysis is incomplete; this report may omit entries.", capabilityError: "Capability analysis failed: {0}", capabilityPathUnknown: "Dynamic paths cannot be decided statically.", capabilityNotGranted: "not granted", capabilityDynamic: "dynamic",
    externalReloaded: "Reloaded externally changed file: {0}", externalDirty: "{0} changed outside Studio. Unsaved edits remain; review before saving.", externalMissing: "{0} was removed outside Studio. The open buffer remains.",
    externalReloadFailed: "Could not reload {0} after an external change: {1}",
    canvasButton: "Canvas", canvasTitle: "Static Zelyra preview", canvasTarget: "Page or view", canvasSample: "Sample data (JSON)", canvasRender: "Refresh preview", canvasSource: "Go to declaration", canvasFrameTitle: "Static page preview", canvasLoading: "Compiler and renderer are preparing the preview …", canvasEmpty: "This project declares no page or view.", canvasError: "Preview unavailable: {0}", canvasSampleError: "Sample data must be a JSON object.", canvasReady: "Rendered statically. No project functions or scripts were executed.", canvasIncomplete: "The project source does not fully compile.",
    canvasThemeToken: "Theme color", canvasThemeColor: "Color value", canvasThemeApply: "Edit source", canvasThemeMissing: "Create zelyra.theme.css in the project to edit colors.", canvasThemeNoHex: "No supported --z-color-* declaration with a hex value was found.", canvasThemeChanged: "Theme color changed in the unsaved source. Save to apply it to the project.", canvasThemeStale: "Theme file changed. Select the color again.", canvasThemeError: "Theme edit failed: {0}",
    runButton: "Run", runButtonTitle: "Run main.zyl or start the web application",
    runOutputTitle: "Program output", runStarting: "Preparing and starting the application …", runSuccess: "Program finished successfully", runFailure: "Program finished with an error", runTimeout: "Execution stopped after 15 seconds", runOutputTruncated: "Output is limited to 64 KiB per stream.", runNoOutput: "No output.", runStdout: "OUTPUT", runStderr: "ERROR", runInputUnavailable: "Studio does not provide interactive input to the program.", runWebReady: "Web application is running locally:", runWebOpen: "Open application in browser", runWebStop: "Stop preview", runWebStopped: "Local preview stopped.",
    runtimeBudgetButton: "Budget", runtimeBudgetTitle: "Runtime budget and recursion", runtimeBudgetEnable: "Enable a step budget for console programs", runtimeBudgetRange: "Step budget (slider: 1,000 to 1,000,000)", runtimeBudgetNumber: "Maximum steps (1 to 100,000,000)", runtimeBudgetWebHint: "The step budget applies to console programs. Web previews keep their own runtime limits.", runtimeRecursionTitle: "Possible recursion", runtimeRecursionNone: "No direct or mutual recursion found in the analyzed function graph.", runtimeRecursionIncomplete: "The function graph is too large; recursion analysis is incomplete.", runtimeRecursionDirect: "Direct recursion", runtimeRecursionMutual: "Mutual recursion", runtimeStackDepth: "Project maximum stack depth: {0} ({1})", runtimeStackConfigured: "configured", runtimeStackDefault: "default", runtimeProfileError: "Could not read runtime profile: {0}", runtimeBudgetInvalid: "Enter an integer step count from 1 to 100,000,000.", runtimeBudgetExceeded: "Step budget exceeded", runCancel: "Cancel execution", runCancelRequested: "Cancellation requested …", runCancelled: "Execution cancelled", runtimeProfileInvalid: "The unsaved source cannot be fully analyzed.",
    logs: "Logs", logsTitle: "Zelyra logs", logsLoading: "Loading logs …", logsUnavailable: "Could not load logs.", logsEmpty: "No program output has been recorded yet.", logsLevel: "Level", logsAll: "All", logsSearch: "Filter text", logsRetention: "Stored locally; bounded rotation", logsCount: "Showing {0} entries",
    taskHelp: "Open contextual help", taskHelpHelp: "Show help for the term at the cursor", taskProperties: "Open file properties",
    taskPropertiesHelp: "Show information and actions for the active file", taskSearch: "Find text", taskSearchHelp: "Search in the active document",
    taskGoToDefinition: "Go to definition", taskGoToDefinitionHelp: "Go to the function or type at the cursor (F12)",
    taskFindReferences: "Find identifier occurrences", taskFindReferencesHelp: "Find exact Zelyra identifiers across the project (Shift+F12)",
    definitionNotFound: "No matching project definition was found.", definitionSaveFirst: "Save this file before jumping so source positions stay accurate.", definitionOpened: "Opened definition: {0}", analysisOverlayLimit: "Unsaved Zelyra files exceed the analysis limit (15 files or 1 MiB).",
    semanticRenameTitle: "Rename function", semanticRenameName: "New function name", semanticRenamePreview: "Change preview", semanticRenameApply: "Apply changes to buffers", semanticRenameReady: "{0} files and {1} occurrences checked. Undo each file before saving.", semanticRenameApplied: "Changed {0} files. Review and save the buffers.", semanticRenameStale: "A source file changed since the preview. Check the rename again.",
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
    renameConfirm: "Rename {0} to {1}? {2}", renameImports: "Imports will be updated in these files:\n{0}", renameNoImports: "No import references are affected.", deleteConfirm: "Permanently delete {0}?",
    fileRenamed: "File renamed: {0}", fileDeleted: "File deleted: {0}",
    switchLanguage: "Switch language", switchFontSize: "Adjust font size", fontSizeDefaultAction: "Default font size (100%)", fontSizeDefaultHelp: "Reset font size of UI elements and editor to default", fontSizeMediumAction: "Medium font size (+15%)", fontSizeMediumHelp: "Increase font size by 15%", fontSizeLargeAction: "Large font size (+30%)", fontSizeLargeHelp: "Increase font size by 30%", themeLightAction: "Use light appearance", themeDarkAction: "Use dark appearance",
    format: "Format", check: "Check project", welcome: "Welcome to Zelyra Studio",
    welcomeHint: "Open a file on the left and work directly in your project.", problems: "Problems",
    noProblems: "No problems found. Your project is ready.", close: "Close panel", loadingFiles: "Loading files",
    toggleExplorer: "Toggle file explorer", search: "Search", handbook: "Handbook", projectFiles: "Project files",
    openFile: "Open file", complete: "Complete", completeHelp: "Show code suggestions (Ctrl+Space)", ready: "Ready", localMode: "Local mode",
    noDocument: "No document open", opened: "Opened {0}", unsaved: "Unsaved changes",
    saved: "Saved {0}", savedAll: "Saved {0} files together", checkRunning: "Checking project …", checkPassed: "Project check passed",
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
    recordPreviewEyebrow: "DATA PREVIEW", recordPreviewTitle: "Browse records",
    recordPreviewHint: "Read only · up to 25 rows per page · loaded only when you choose.",
    recordPreviewLoad: "Load records", recordPreviewPrevious: "Previous", recordPreviewNext: "Next",
    recordPreviewIdle: "Choose a table, then explicitly load its preview.", recordPreviewLoading: "Reading records …",
    recordPreviewPage: "Rows {0}–{1}", recordPreviewEmpty: "No records on this page.",
    recordPreviewIncomplete: "The preview is bounded. Long values or additional columns were shortened.",
    recordPreviewFailed: "Could not load records.",
    chartEyebrow: "BUSINESS CHART", chartTitle: "Make your data clear", chartIntro: "Choose a grouping and a measure. Studio queries only the selected table.",
    chartName: "Chart title", chartGroup: "Group by", chartAggregation: "Measure", chartCount: "Record count", chartSum: "Sum", chartAvg: "Average", chartMin: "Minimum", chartMax: "Maximum", chartMeasure: "Value from field", chartKind: "Display", chartBars: "Bars", chartLine: "Line", chartFilterColumn: "Text filter field", chartFilterValue: "Exact filter value", chartNoFilter: "No filter", chartPeriodColumn: "Date field", chartPeriodFrom: "From inclusive", chartPeriodTo: "To inclusive", chartPreview: "Preview chart", chartSave: "Save chart", chartPreviewRunning: "Calculating a safe preview …", chartPreviewReady: "Preview ready · up to 20 groups", chartPreviewFailed: "Could not calculate the chart.", chartSaved: "Chart configuration saved: {0}", chartSaveFailed: "Could not save the chart.", chartSaveFirst: "Preview the current chart first.", chartTruncated: "The preview shows at most 20 groups.", chartEmpty: "No grouped data found.", chartGroupLabel: "Group", chartValueLabel: "Value", chartAccessible: "{0} grouped by {1}", chartDefaultTitle: "Report {0}", chartInvalidNumber: "Non-numeric measure value",
    presentationEyebrow: "PRESENTATION", presentationTitle: "Share business results", presentationIntro: "Add reviewed charts and open a print-ready, locally generated PDF preview.", presentationName: "Title", presentationNote: "Introduction", presentationEmpty: "No charts yet", presentationCount: "{0} chart(s) · {1}", presentationAdd: "Add to presentation", presentationRemove: "Remove", presentationSave: "Save presentation", presentationSaved: "Presentation saved: {0}", presentationLoaded: "Presentation loaded: {0}", presentationSaveFailed: "Could not save the presentation: {0}", presentationPdf: "Print view / PDF", presentationNeedCharts: "Add at least one chart.", presentationPrintFailed: "Could not open the presentation preview.", presentationLoadFailed: "A chart could not be loaded for the presentation.", presentationKicker: "ZELYRA BUSINESS REPORT", presentationFooter: "Created locally with Zelyra Studio", presentationNumber: "Slide {0} of {1}",
    savedChart: "Saved chart", savedChartAdd: "Load and add", savedPresentation: "Saved presentation", savedPresentationLoad: "Open presentation", reportFilesEmpty: "No saved reports found.", chartInvalidConfig: "The saved chart file is invalid or unsupported.", chartAdded: "Saved chart added to the presentation.", reportFileConflict: "A file with a different content type already exists at this path.", presentationInvalidConfig: "The saved presentation is invalid or unsupported.",
    createDatabase: "Create MariaDB database", databaseTargetLoading: "Checking project target …",
    databaseTargetMissing: "No supported MariaDB target with a configured connection.",
    databaseTargetReady: "MariaDB target {0} · connection configured",
    databaseCreateConfirm: "Create the MariaDB database {0}, or ensure that it exists? Credentials are used only on the local server.",
    databaseCreated: "Database exists. Review and apply the schema plan separately.",
    tableNotDeclared: "This live table is not declared in Zelyra yet. Add a reviewed table declaration first.",
    existingCrud: "A CRUD resource already exists for this table.",
    previewRunning: "Checking generated source …", previewPassed: "Compiler check passed · change not yet applied",
    previewFailed: "Compiler check failed", previewRequired: "Create a valid preview first.",
    applySuccess: "CRUD change applied", selectOneField: "Choose at least one field.",
    schemaColumnTitle: "Edit table columns", schemaColumnHint: "Creates and checks a Zelyra source change. It does not change the database.",
    schemaColumnOperation: "Action", schemaColumnAdd: "Add", schemaColumnEdit: "Edit", schemaColumnExisting: "Existing field",
    schemaColumnName: "Field name", schemaColumnType: "Data type", schemaColumnRequired: "Required",
    schemaColumnPreview: "Check change", schemaColumnApply: "Apply source change",
    schemaColumnRunning: "Checking table declaration …", schemaColumnPassed: "Compiler check passed · database unchanged",
    schemaColumnFailed: "Compiler check failed", schemaColumnConfirm: "Change field {0} in project source? The database stays unchanged. Review the schema plan afterwards.",
    schemaColumnApplied: "Source updated. Review `zelyra db plan main.zyl` before applying a database change.",
    table: "Table", field: "Field", moduleUnknown: "Source module unknown", selectTarget: "Choose source file"
  }
};
let language = localStorage.getItem("zelyra-editor-language") === "en" ? "en" : "de";
function t(key, ...values) {
  return values.reduce(
    (message, value, index) => message.replaceAll(`{${index}}`, value ?? ""),
    messages[language][key] || key,
  );
}
function updateThemeControl() {
  const label = t(root.dataset.theme === "dark" ? "themeLightAction" : "themeDarkAction");
  $("theme-button").setAttribute("aria-label", label);
  $("theme-button").title = label;
}
function currentFontSize() {
  const size = root.dataset.fontSize;
  return size === "medium" || size === "large" ? size : "default";
}
function updateFontSizeControl() {
  const btn = $("font-size-button");
  if (!btn) return;
  const size = currentFontSize();
  if (size === "large") {
    btn.textContent = "aA⁺⁺";
    const label = `${t("switchFontSize")} (${t("fontSizeLargeAction")})`;
    btn.setAttribute("aria-label", label);
    btn.title = label;
  } else if (size === "medium") {
    btn.textContent = "aA⁺";
    const label = `${t("switchFontSize")} (${t("fontSizeMediumAction")})`;
    btn.setAttribute("aria-label", label);
    btn.title = label;
  } else {
    btn.textContent = "aA";
    const label = `${t("switchFontSize")} (${t("fontSizeDefaultAction")})`;
    btn.setAttribute("aria-label", label);
    btn.title = label;
  }
}
function setFontSize(nextSize) {
  if (nextSize === "medium" || nextSize === "large") {
    root.dataset.fontSize = nextSize;
    try { localStorage.setItem("zelyra-editor-font-size", nextSize); } catch {}
  } else {
    delete root.dataset.fontSize;
    try { localStorage.removeItem("zelyra-editor-font-size"); } catch {}
  }
  updateFontSizeControl();
  if (editor) editor.dispatch({ effects: editorTheme.reconfigure(createZelyraTheme()) });
}
function cycleFontSize() {
  const current = currentFontSize();
  if (current === "default") setFontSize("medium");
  else if (current === "medium") setFontSize("large");
  else setFontSize("default");
}
function setLanguage(next) {
  language = next;
  document.documentElement.lang = language;
  $("language-button").textContent = language.toUpperCase();
  updateThemeControl();
  updateFontSizeControl();
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
  "fn let mut if else match for in step while loop break continue return type record " +
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
  for: ["Wiederholung", "Verarbeitet jedes Listenelement oder einen Integer-Bereich in einem Block.", "for i in 1..21 step 2 { ... }", "Das Bereichsende ist exklusiv; die Schrittweite kann positiv oder negativ sein."],
  step: ["Schrittweite", "Legt den Abstand einer Integer-Bereichsschleife fest.", "for i in 1..21 step 2 { ... }", "Positive Werte zählen aufwärts, negative abwärts. Null ist ungültig."],
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
  for: ["iteration", "Processes each list item or integer in a range inside a block.", "for i in 1..21 step 2 { ... }", "Range ends are exclusive; the step may be positive or negative."],
  step: ["range step", "Sets the distance between values in an integer range loop.", "for i in 1..21 step 2 { ... }", "Positive values count upward, negative values downward. Zero is invalid."],
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

const editorTheme = new Compartment();
function createZelyraTheme() {
  const size = currentFontSize();
  const fontSize = size === "large" ? "18px" : size === "medium" ? "16px" : "14px";
  return EditorView.theme({
  "&": { height: "100%", fontSize, backgroundColor: "var(--editor-bg)", color: "var(--ink)" },
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
  }, { dark: document.documentElement.dataset.theme !== "light" });
}
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
  const response = await fetch(`/__zelyra/editor/api/${path}`, { ...options, headers, cache: "no-store", credentials: "omit", redirect: "error" });
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
function analysisRequest(path, source, version = liveCheckSequence) {
  const overlays = [];
  let bytes = typeof source === "string" ? new TextEncoder().encode(source).length : 0;
  for (const [otherPath, tab] of tabs) {
    if (otherPath === path || !otherPath.endsWith(".zyl") || !isDirty(tab, otherPath)) continue;
    const text = tabState(otherPath, tab).doc.toString();
    bytes += new TextEncoder().encode(text).length;
    overlays.push({ path: otherPath, source: text, version: tab.version || 0 });
  }
  if (overlays.length >= 16 || bytes > 1_048_576) throw new Error(t("analysisOverlayLimit"));
  return { path, source, version, overlays };
}

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
  if (word?.text === "_" && activePath?.endsWith(".zyl")) {
    const path = activePath;
    let request;
    try { request = analysisRequest(path, source); }
    catch { return null; }
    return api("typed-hole", { method: "POST", body: JSON.stringify({
      ...request, offset: context.pos,
    }) }).then((result) => {
      if (!result.found || activePath !== path || editor?.state.doc.toString() !== source) return null;
      const { start, end } = result.editorSpan;
      return {
        from: start, to: end,
        filter: false,
        options: (result.suggestions || []).map((item) => ({
          label: item.expression,
          apply: item.expression,
          type: item.kind === "function" ? "function" : item.kind === "value" ? "variable" : "text",
          detail: item.type || result.expectedType || "",
          info: item.contractReview
            ? (language === "de" ? "Contracts müssen geprüft werden." : "Contracts need review.")
            : undefined,
        })),
      };
    }).catch(() => null);
  }
  const from = word ? word.from : context.pos;
  const basicOptions = [
    ...[...keywords].map((label) => ({ label, type: "keyword" })),
    { label: "main", type: "function", detail: t("symbolEntrypoint") },
  ];
  const projectOptions = Promise.all([
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
    return [...localOptions, ...projectOptions];
  });
  const fast = { from, options: basicOptions };
  const complete = projectOptions.then((options) => {
    const merged = new Map(basicOptions.map((item) => [item.label, item]));
    for (const item of options) merged.set(item.label, item);
    return { from, options: [...merged.values()] };
  });
  const fallback = new Promise((resolve) => setTimeout(() => resolve(fast), 300));
  return Promise.race([complete, fallback]);
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
  renderUiPropertyInspector(editor?.state, path);
}

function matchingBrace(source, opening) {
  let depth = 0;
  let quote = "";
  let escaped = false;
  for (let index = opening; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = "";
      continue;
    }
    if (character === '"' || character === "'") quote = character;
    else if (character === "{") depth += 1;
    else if (character === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function componentAtCursor(state) {
  if (!state) return null;
  const source = state.doc.toString();
  const position = state.selection.main.head;
  const start = source.lastIndexOf("<", position);
  if (start < 0 || source.lastIndexOf(">", position) > start) return null;
  const end = source.indexOf(">", position);
  if (end < 0) return null;
  const openingTag = source.slice(start, end + 1);
  const name = openingTag.match(/^<([A-Z][A-Za-z0-9_]*)\b/)?.[1];
  if (!name || !zelyraUiComponentMetadata[name]) return null;
  const attributes = new Map();
  const expression = /(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"([^"]*)"/g;
  for (const match of openingTag.matchAll(expression)) attributes.set(match[1], match[2]);
  return { name, start, end: end + 1, openingTag, attributes };
}

function formFieldAtCursor(state) {
  if (!state) return null;
  const source = state.doc.toString();
  const position = state.selection.main.head;
  const declaration = /\bfield\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*:\s*([^{}\n]+?))?\s*\{/g;
  for (const match of source.matchAll(declaration)) {
    const opening = match.index + match[0].lastIndexOf("{");
    const closing = matchingBrace(source, opening);
    if (closing < 0 || position < match.index || position > closing) continue;
    const body = source.slice(opening + 1, closing);
    const get = (name) => {
      const result = body.match(new RegExp(`\\b${name}\\s*:\\s*("[^"]*"|'[^']*'|[A-Za-z_][A-Za-z0-9_]*)`));
      return result?.[1]?.replace(/^["']|["']$/g, "") || null;
    };
    return {
      name: match[1], type: match[2]?.trim() || "automatisch aus Ressource",
      start: match.index, opening, closing, body, appearance: get("appearance"), widget: get("widget"),
      label: get("label"), hasRequired: /\brequired\b/.test(body),
      line: state.doc.lineAt(match.index).number,
    };
  }
  return null;
}

function appendPropertyRow(list, name, type, value) {
  const term = document.createElement("dt");
  term.textContent = `${name}: ${type}`;
  const description = document.createElement("dd");
  description.textContent = value;
  list.append(term, description);
}

function showAppearanceEditor(target, currentValue, kind) {
  const editorPanel = $("assistant-ui-appearance-editor");
  const select = $("assistant-ui-appearance");
  const button = $("assistant-ui-appearance-apply");
  const label = $("assistant-ui-appearance-label");
  editorPanel.hidden = false;
  select.replaceChildren();
  for (const choice of zelyraUiAppearanceValues) {
    const option = document.createElement("option");
    option.value = choice.value;
    option.textContent = language === "de" ? choice.de : choice.en;
    select.append(option);
  }
  select.value = currentValue || "default";
  label.textContent = language === "de"
    ? "Erscheinung explizit überschreiben"
    : "Override appearance explicitly";
  button.textContent = language === "de" ? "Übernehmen" : "Apply";
  button.onclick = () => {
    const value = select.value;
    const state = editor?.state;
    if (!state || !editor || !target) return;
    if (kind === "component") {
      const attribute = /(?:^|\s)appearance\s*=\s*"[^"]*"/.exec(target.openingTag);
      if (attribute) {
        const quoteStart = target.start + attribute.index + attribute[0].indexOf('"') + 1;
        const quoteEnd = state.doc.toString().indexOf('"', quoteStart);
        if (quoteEnd < quoteStart) return;
        editor.dispatch({ changes: { from: quoteStart, to: quoteEnd, insert: value } });
      } else {
        const insertion = target.end - (target.openingTag.trimEnd().endsWith("/>") ? 2 : 1);
        editor.dispatch({ changes: { from: insertion, insert: ` appearance="${value}"` } });
      }
    } else {
      const source = state.doc.toString();
      const body = source.slice(target.opening + 1, target.closing);
      const existing = /\bappearance\s*:\s*("[^"]*"|'[^']*'|[A-Za-z_][A-Za-z0-9_]*)/.exec(body);
      if (existing) {
        const valueStart = target.opening + 1 + existing.index + existing[0].lastIndexOf(existing[1]);
        editor.dispatch({ changes: { from: valueStart, to: valueStart + existing[1].length, insert: value } });
      } else {
        const closeLine = state.doc.lineAt(target.closing);
        const beforeBrace = source.slice(closeLine.from, target.closing);
        const indent = beforeBrace.match(/^\s*/)?.[0] || "";
        const insertion = beforeBrace.trim()
          ? `\n${indent}    appearance: ${value}\n${indent}`
          : `${indent}    appearance: ${value}\n`;
        editor.dispatch({ changes: { from: beforeBrace.trim() ? target.closing : closeLine.from, insert: insertion } });
      }
    }
    announce(language === "de" ? `Erscheinung ${value} übernommen.` : `Appearance ${value} applied.`, "success");
  };
}

function renderUiPropertyInspector(state, path) {
  const panel = $("assistant-ui-component-properties");
  const details = $("assistant-ui-property-details");
  const heading = $("assistant-ui-component-title");
  const summary = $("assistant-ui-component-summary");
  const slots = $("assistant-ui-component-slots");
  const appearanceEditor = $("assistant-ui-appearance-editor");
  if (!panel || !state || !path?.endsWith(".zyl")) {
    if (panel) panel.hidden = true;
    return;
  }
  const component = componentAtCursor(state);
  const field = component ? null : formFieldAtCursor(state);
  if (!component && !field) {
    panel.hidden = true;
    return;
  }
  panel.hidden = false;
  details.replaceChildren();
  slots.replaceChildren();
  appearanceEditor.hidden = true;
  if (component) {
    const metadata = zelyraUiComponentMetadata[component.name];
    heading.textContent = `<${component.name}>`;
    summary.textContent = metadata.automatic;
    for (const prop of metadata.properties) {
      const value = component.attributes.get(prop.name);
      const defaultValue = zelyraUiDefaultAppearance(component.name, prop.name) ?? prop.defaultValue;
      const stateText = value !== undefined
        ? `${language === "de" ? "explizit" : "explicit"}: ${value}`
        : `${language === "de" ? "automatisch" : "automatic"}: ${prop.required ? (language === "de" ? "erforderlich" : "required") : defaultValue}`;
      appendPropertyRow(details, prop.name, prop.type, stateText);
    }
    for (const slot of metadata.slots) {
      const item = document.createElement("li"); item.textContent = slot; slots.append(item);
    }
    if (metadata.properties.some((prop) => prop.name === "appearance")) {
      showAppearanceEditor(component, component.attributes.get("appearance"), "component");
    }
  } else {
    heading.textContent = language === "de" ? `Formularfeld · ${field.name}` : `Form field · ${field.name}`;
    const typeText = field.widget
      ? `${field.type} · ${language === "de" ? "Widget" : "widget"}: ${field.widget}`
      : field.type;
    summary.textContent = language === "de"
      ? "Die Anzeigevariante steuert nur das Aussehen. Fehlerzustand und zulässige Eingaben bestimmt die serverseitige Formularprüfung."
      : "Appearance controls only presentation. Server form validation controls errors and accepted values.";
    appendPropertyRow(details, "type", "automatisch", typeText);
    appendPropertyRow(details, "label", "String?", field.label || (language === "de" ? `automatisch aus ${field.name}` : `defaults to ${field.name}`));
    appendPropertyRow(details, "required", "Bool", field.hasRequired ? (language === "de" ? "explizit gesetzt" : "explicitly set") : (language === "de" ? "nicht im Feld gesetzt; Ressource kann zusätzlich einschränken" : "not set on field; resource may add a constraint"));
    appendPropertyRow(details, "appearance", "UiAppearance", field.appearance || (language === "de" ? "automatisch: default" : "automatic: default"));
    appendPropertyRow(details, "invalid", "Bool", language === "de" ? "wird aus dem tatsächlichen Serverfehler gesetzt" : "set from the actual server validation error");
    for (const itemText of [
      language === "de" ? "Syntax: appearance: success" : "Syntax: appearance: success",
      language === "de" ? "Servervalidierung bleibt unabhängig." : "Server validation remains independent.",
    ]) { const item = document.createElement("li"); item.textContent = itemText; slots.append(item); }
    showAppearanceEditor(field, field.appearance, "field");
  }
  const slotsTitle = $("assistant-ui-slots-title");
  slotsTitle.textContent = language === "de" ? "Slots / Hinweise" : "Slots / notes";
  $("assistant-ui-slots-list").hidden = slots.childElementCount === 0;
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
  ["pdfPreview", "pdfPreviewTitle", ["pdf", "document", "invoice", "print"], () => void openPdfPreview()],
  ["outboxButton", "outboxTitle", ["outbox", "mail", "email", "queue"], openOutbox],
  ["capabilityButton", "capabilityTitle", ["capability", "permission", "sandbox", "security"], openCapabilityMatrix],
  ["canvasButton", "canvasTitle", ["canvas", "preview", "page", "view"], openCanvas],
  ["taskDatabase", "taskDatabaseHelp", ["database", "table", "relation", "schema"], () => openDatabaseWorkbench()],
  ["taskHelp", "taskHelpHelp", ["help", "documentation", "f1"], () => toggleAssistant("help")],
  ["taskProperties", "taskPropertiesHelp", ["properties", "file", "settings"], () => toggleAssistant("properties")],
  ["taskSearch", "taskSearchHelp", ["find", "search", "text"], () => findInDocument()],
  ["taskGoToDefinition", "taskGoToDefinitionHelp", ["definition", "symbol", "function", "type", "f12"], () => void goToDefinition()],
  ["taskFindReferences", "taskFindReferencesHelp", ["references", "usages", "occurrences", "symbol", "shift f12"], () => void findIdentifierOccurrences()],
  ["projectSearch", "projectSearchHelp", ["project", "search", "find", "all files"], () => openProjectSearch()],
  ["fontSizeDefaultAction", "fontSizeDefaultHelp", ["font", "size", "zoom", "schriftgröße", "default", "standard", "100"], () => setFontSize("default")],
  ["fontSizeMediumAction", "fontSizeMediumHelp", ["font", "size", "zoom", "schriftgröße", "medium", "mittel", "115"], () => setFontSize("medium")],
  ["fontSizeLargeAction", "fontSizeLargeHelp", ["font", "size", "zoom", "schriftgröße", "large", "groß", "130"], () => setFontSize("large")],
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
  semanticReferenceQuery = null;
  if (dirtyCount()) { announce(t("projectSearchSaveFirst"), "warning"); return; }
  const dialog = $("project-search");
  if (!dialog.open) dialog.showModal();
  $("project-search-title").textContent = t("projectSearchTitle");
  $("project-search-input").placeholder = t("projectSearchPlaceholder");
  $("project-search-input").readOnly = false;
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
  const selection = editor.state.selection.main;
  const wordRange = selection.empty ? editor.state.wordAt(selection.head) : selection;
  const name = wordRange ? editor.state.sliceDoc(wordRange.from, wordRange.to).trim() : "";
  if (!name) {
    announce(t("definitionNotFound"), "warning");
    return;
  }
  projectSearchMode = "references";
  try {
    semanticReferenceQuery = {
      ...analysisRequest(activePath, editor.state.doc.toString()),
      offset: wordRange.from,
      action: "references",
    };
  } catch (error) { announce(error.message, "warning"); return; }
  const dialog = $("project-search");
  if (!dialog.open) dialog.showModal();
  $("project-search-title").textContent = t("referencesTitle");
  $("project-search-input").placeholder = t("referencesPlaceholder");
  $("project-search-input").readOnly = true;
  $("project-search-input").value = name;
  $("project-search-results").replaceChildren();
  $("project-search-empty").hidden = true;
  void performProjectSearch();
}

async function performProjectSearch() {
  if (projectSearchMode !== "references" && dirtyCount()) { announce(t("projectSearchSaveFirst"), "warning"); $("project-search").close(); return; }
  const query = $("project-search-input").value.trim();
  if (!query) return;
  const results = $("project-search-results");
  results.replaceChildren();
  const empty = $("project-search-empty");
  empty.textContent = t("projectSearchLoading");
  empty.hidden = false;
  try {
    const response = projectSearchMode === "references"
      ? await api("semantic", { method: "POST", body: JSON.stringify(semanticReferenceQuery) })
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
  const wordRange = editor.state.wordAt(editor.state.selection.main.head);
  const word = wordRange ? editor.state.sliceDoc(wordRange.from, wordRange.to) : "";
  if (!word) { announce(t("definitionNotFound"), "warning"); return; }
  try {
    const source = editor.state.doc.toString();
    const requestedPath = activePath;
    const result = await api("semantic", { method: "POST", body: JSON.stringify({
      ...analysisRequest(requestedPath, source), offset: wordRange.from, action: "definition",
    }) });
    if (activePath !== requestedPath || editor.state.doc.toString() !== source) return;
    if (!result.found) { announce(t("definitionNotFound"), "warning"); return; }
    if (result.path !== activePath) await openFile(result.path);
    if (activePath !== result.path || !editor) return;
    editor.dispatch({ selection: { anchor: result.start, head: result.end }, scrollIntoView: true });
    editor.focus();
    announce(t("definitionOpened", word), "success");
  } catch (error) { announce(error.message, "error"); }
}

let semanticRenameContext = null;
let semanticRenamePlan = null;

function openSemanticRename() {
  if (!editor || !activePath?.endsWith(".zyl")) return;
  const selected = editor.state.selection.main;
  const range = selected.empty ? editor.state.wordAt(selected.head) : selected;
  if (!range) { announce(t("definitionNotFound"), "warning"); return; }
  try {
    semanticRenameContext = {
      request: { ...analysisRequest(activePath, editor.state.doc.toString()),
        offset: range.from, action: "rename-preview" },
      oldName: editor.state.sliceDoc(range.from, range.to),
    };
  } catch (error) { announce(error.message, "warning"); return; }
  semanticRenamePlan = null;
  $("semantic-rename-name").value = semanticRenameContext.oldName;
  $("semantic-rename-status").textContent = "";
  $("semantic-rename-changes").replaceChildren();
  $("semantic-rename-apply-button").disabled = true;
  $("semantic-rename-dialog").showModal();
  $("semantic-rename-name").focus();
  $("semantic-rename-name").select();
}

async function previewSemanticRename() {
  if (!semanticRenameContext) return;
  const newName = $("semantic-rename-name").value.trim();
  semanticRenamePlan = null;
  $("semantic-rename-apply-button").disabled = true;
  const target = $("semantic-rename-changes");
  target.replaceChildren();
  try {
    const result = await api("semantic", { method: "POST", body: JSON.stringify({
      ...semanticRenameContext.request, newName,
    }) });
    if (!$("semantic-rename-dialog").open || $("semantic-rename-name").value.trim() !== newName) return;
    if (!result.found) throw new Error(t("definitionNotFound"));
    semanticRenamePlan = result;
    let count = 0;
    for (const change of result.changes || []) {
      const section = document.createElement("section");
      const heading = document.createElement("strong");
      heading.textContent = `${change.path} · ${change.edits.length}`;
      const before = document.createElement("pre");
      before.textContent = change.edits.map((edit) => {
        const line = change.before.slice(0, edit.start).split("\n").length;
        return `${line}: ${change.before.slice(edit.start, edit.end)} → ${edit.newText}`;
      }).join("\n");
      section.append(heading, before);
      target.append(section);
      count += change.edits.length;
    }
    $("semantic-rename-status").textContent = t("semanticRenameReady", result.changes.length, count);
    $("semantic-rename-apply-button").disabled = false;
  } catch (error) { $("semantic-rename-status").textContent = error.message; }
}

async function applySemanticRename() {
  const plan = semanticRenamePlan;
  if (!plan) return;
  $("semantic-rename-apply-button").disabled = true;
  const originalPath = activePath;
  const applied = [];
  try {
    for (const change of plan.changes) {
      await openFile(change.path);
      const tab = tabs.get(change.path);
      if (!tab || tabState(change.path, tab).doc.toString() !== change.before) {
        throw new Error(t("semanticRenameStale"));
      }
    }
    for (const change of plan.changes) {
      activate(change.path);
      editor.dispatch({ changes: change.edits.map((edit) => ({
        from: edit.start, to: edit.end, insert: edit.newText,
      })) });
      applied.push(change);
      if (editor.state.doc.toString() !== change.after) throw new Error(t("semanticRenameStale"));
    }
    $("semantic-rename-dialog").close();
    announce(t("semanticRenameApplied", plan.changes.length), "success");
    if (originalPath && tabs.has(originalPath)) activate(originalPath);
    semanticRenamePlan = null;
  } catch (error) {
    $("semantic-rename-status").textContent = error.message;
    for (const change of applied.reverse()) {
      const tab = tabs.get(change.path);
      if (tab) {
        activate(change.path);
        editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: change.before } });
      }
    }
    if (originalPath && tabs.has(originalPath)) activate(originalPath);
  }
}

function createView(path, state) {
  const extensions = [
    lineNumbers(), highlightActiveLineGutter(), foldGutter(), verificationGutter, undoRedoHistory(), drawSelection(),
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
      { key: "Mod-Space", run: startCompletion },
      { key: "F12", run: () => { void goToDefinition(); return true; } },
      { key: "F2", run: () => { openSemanticRename(); return true; } },
      { key: "Shift-F12", run: () => { void findIdentifierOccurrences(); return true; } },
      { key: "F1", run: () => { toggleAssistant("help"); return true; } },
      ...defaultKeymap, ...historyKeymap, ...searchKeymap, ...foldKeymap,
    ]),
    EditorView.updateListener.of((update) => {
      const tab = tabs.get(path);
      if (!tab) return;
      tab.state = update.state;
      if (update.docChanged) {
        tab.version = (tab.version || 0) + 1;
        verificationResults = [];
        pipelineMapCache = null;
        renderTabs();
        scheduleCanvasRefresh();
        announce(t("unsaved"), "warning");
        queueMicrotask(() => {
          if (editor === update.view) {
            editor.dispatch(setDiagnostics(editor.state, []));
            editor.dispatch({ effects: verificationRefresh.of(null) });
          }
        });
      }
      if (update.selectionSet || update.docChanged) {
        renderContextHelp(update.state, path);
        renderPipelineBreadcrumbs();
      }
      if ((update.selectionSet || update.docChanged) && !$("assistant-properties-view").hidden) {
        renderUiPropertyInspector(update.state, path);
      }
    }),
    EditorView.lineWrapping,
    editorTheme.of(createZelyraTheme()),
    EditorView.contentAttributes.of({ "aria-label": `${path} im Zelyra-Webeditor` }),
  ];
  return new EditorView({
    state: state.update({ effects: StateEffect.reconfigure.of(extensions) }).state,
    parent: editorRoot,
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

async function refreshExternallyChangedTabs(changedPath) {
  const paths = changedPath ? [changedPath] : [...tabs.keys()];
  for (const path of paths) {
    const tab = tabs.get(path);
    if (!tab) continue;
    if (isDirty(tab, path)) {
      announce(t("externalDirty", path), "warning");
      continue;
    }
    const expectedHash = tab.hash;
    let file;
    try { file = await api(`file?path=${encodeURIComponent(path)}`); }
    catch (error) {
      announce(error.message.includes("file not found") ? t("externalMissing", path)
        : t("externalReloadFailed", path, error.message), "warning");
      continue;
    }
    if (tabs.get(path) !== tab || isDirty(tab, path) || tab.hash !== expectedHash || file.hash === expectedHash) continue;
    const wasActive = activePath === path;
    if (wasActive && editor) { editor.destroy(); editor = null; }
    tab.saved = file.content;
    tab.hash = file.hash;
    tab.state = EditorState.create({ doc: file.content });
    if (wasActive) {
      editor = createView(path, tab.state);
      setVerificationResults([]);
      renderContextHelp(editor.state, path);
    }
    symbolCache.delete(path);
    renderTabs();
    announce(t("externalReloaded", path));
  }
}

function activate(path) {
  if (!tabs.has(path)) return;
  if (editor) { tabs.get(activePath).state = editor.state; editor.destroy(); }
  activePath = path;
  pipelineMapCache = null;
  renderPipelineBreadcrumbs();
  verificationResults = [];
  const tab = tabs.get(path);
  editor = createView(path, tab.state);
  $("active-file").textContent = path;
  $("language-label").textContent = path.endsWith(".zyl") ? "Zelyra" : path.split(".").pop().toUpperCase();
  $("empty-state").hidden = true;
  renderTabs();
  editor.focus();
  scheduleLiveCheck(path);
}

function closeTab(path) {
  const tab = tabs.get(path);
  if (isDirty(tab, path) && !confirm(t("closeUnsaved", path))) return;
  if (activePath === path && editor) { editor.destroy(); editor = null; }
  tabs.delete(path);
  if (activePath === path) {
    activePath = null;
    pipelineMapCache = null;
    renderPipelineBreadcrumbs();
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
  const files = [...tabs].filter(([path, tab]) => isDirty(tab, path)).map(([path, tab]) => ({
    path, content: tabState(path, tab).doc.toString(), baseHash: tab.hash,
  }));
  if (!files.length) return true;
  try {
    const result = await api("file-save-batch", { method: "POST", body: JSON.stringify({ files }) });
    for (const file of result.files || []) {
      const tab = tabs.get(file.path);
      if (!tab) continue;
      tab.saved = file.content;
      tab.hash = file.hash;
      symbolCache.delete(file.path);
    }
    renderTabs();
    announce(t("savedAll", files.length), "success");
    return true;
  } catch (error) { announce(error.message, "error"); return false; }
}

async function renameActiveFile() {
  const path = activePath;
  if (!path || path === "main.zyl" || path === "zelyra.toml") return;
  const newPath = prompt(t("renamePrompt"), path);
  if (!newPath || newPath === path) return;
  if (!(await saveAll())) return;
  try {
    const tab = tabs.get(path);
    const request = { path, newPath, baseHash: tab.hash };
    const preview = await api("file-rename-preview", { method: "POST", body: JSON.stringify(request) });
    const affected = preview.updatedImports || [];
    const details = affected.length ? t("renameImports", affected.join("\n")) : t("renameNoImports");
    if (!confirm(t("renameConfirm", path, newPath, details))) return;
    const result = await api("file-rename", { method: "POST", body: JSON.stringify({ ...request, planId: preview.planId }) });
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
  renderSavedReportChoices();
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
    state.textContent = databaseMap.live_connected === false
      ? `${t("schemaLoaded")} (deklariert) · ${databaseMap.backend || ""}`
      : `${t("schemaLoaded")} · ${databaseMap.backend || ""}`;
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

async function loadDatabaseTarget() {
  const status = $("database-target-state");
  const button = $("database-create");
  status.textContent = t("databaseTargetLoading");
  button.hidden = true;
  try {
    const target = await api("database-target");
    if (target.creationSupported) {
      status.textContent = t("databaseTargetReady", target.database || "main");
      button.hidden = false;
    } else {
      status.textContent = t("databaseTargetMissing");
    }
  } catch (error) {
    status.textContent = error.message;
  }
}

async function createDatabaseFromStudio() {
  const status = $("database-target-state");
  const button = $("database-create");
  try {
    const target = await api("database-target");
    if (!target.creationSupported) throw new Error(t("databaseTargetMissing"));
    if (!window.confirm(t("databaseCreateConfirm", target.database || "main"))) return;
    button.disabled = true;
    const result = await api("database-create", {
      method: "POST",
      body: JSON.stringify({ confirm: true }),
    });
    status.textContent = t("databaseCreated");
    announce(result.message || t("databaseCreated"), "success");
    databaseMap = null;
    await loadDatabaseSchema();
  } catch (error) {
    status.textContent = error.message;
    announce(error.message, "error");
  } finally {
    button.disabled = false;
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
  resetDatabaseRecordPreview();
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
  renderSchemaColumnEditor(selectedDatabaseTable);
  configureChartBuilder(selectedDatabaseTable);
  renderCrudBuilder(selectedDatabaseTable);
  renderDatabaseTables();
  renderDatabaseGraph();
}

function resetSchemaColumnPreview() {
  schemaColumnPreview = null;
  $("schema-column-preview-grid").hidden = true;
  $("schema-column-diagnostics").replaceChildren();
  $("schema-column-diagnostics").hidden = true;
  $("schema-column-apply").disabled = true;
  $("schema-column-state").textContent = "";
  $("schema-column-state").dataset.kind = "";
}

function renderSchemaColumnEditor(table) {
  resetSchemaColumnPreview();
  const owner = table.module;
  const controls = $("schema-column-editor");
  controls.hidden = !owner || table.status === "live_unmapped";
  $("schema-column-name").value = "";
  const existing = $("schema-column-existing");
  existing.replaceChildren();
  for (const column of table.declared_columns || []) {
    if (column.primary_key || column.auto_increment || column.unique) continue;
    const option = document.createElement("option"); option.value = column.name; option.textContent = column.name; existing.append(option);
  }
  $("schema-column-operation").value = "add";
  $("schema-column-existing-wrap").hidden = true;
  $("schema-column-name").readOnly = false;
  $("schema-column-type").value = "String(120)";
  $("schema-column-required").checked = false;
}

function updateSchemaColumnOperation() {
  const editing = $("schema-column-operation").value === "edit";
  $("schema-column-existing-wrap").hidden = !editing;
  $("schema-column-name").readOnly = editing;
  if (editing) updateSchemaColumnField();
  resetSchemaColumnPreview();
}

function updateSchemaColumnField() {
  const field = $("schema-column-existing").value;
  const column = (selectedDatabaseTable?.declared_columns || []).find((item) => item.name === field);
  if (!column) return;
  $("schema-column-name").value = column.name;
  $("schema-column-required").checked = column.zelyra_required === true;
  const type = column.zelyra_type;
  if (type) {
    const select = $("schema-column-type");
    let option = [...select.options].find((item) => item.value === type);
    if (!option) { option = document.createElement("option"); option.value = type; option.textContent = type; select.append(option); }
    select.value = type;
  }
}

function schemaColumnRequest() {
  if (!selectedDatabaseTable?.module) throw new Error(t("schemaColumnFailed"));
  const column = $("schema-column-name").value.trim();
  if (!column) throw new Error(`${t("schemaColumnName")} is required`);
  return {
    path: selectedDatabaseTable.module,
    table: selectedDatabaseTable.name,
    column,
    operation: $("schema-column-operation").value,
    type: $("schema-column-type").value,
    required: $("schema-column-required").checked,
  };
}

async function previewSchemaColumn() {
  resetSchemaColumnPreview();
  $("schema-column-state").textContent = t("schemaColumnRunning");
  $("schema-column-state").dataset.kind = "pending";
  try {
    const request = schemaColumnRequest();
    const file = await api(`file?path=${encodeURIComponent(request.path)}`);
    request.baseHash = file.hash;
    const result = await api("schema-column-preview", { method: "POST", body: JSON.stringify(request) });
    $("schema-column-diff").textContent = result.diff || "";
    $("schema-column-impact").textContent = JSON.stringify(result.impact || {}, null, 2);
    $("schema-column-preview-grid").hidden = false;
    const diagnostics = $("schema-column-diagnostics");
    diagnostics.replaceChildren();
    for (const item of result.diagnostics || []) {
      const row = document.createElement("div"); row.className = "crud-diagnostic";
      const code = document.createElement("strong"); code.textContent = item.code || "Compiler";
      const message = document.createElement("span"); message.textContent = item.message || "";
      row.append(code, message); diagnostics.append(row);
    }
    diagnostics.hidden = !diagnostics.childElementCount;
    if (result.success) {
      schemaColumnPreview = { ...request, candidateHash: result.candidateHash };
      $("schema-column-apply").disabled = false;
      $("schema-column-state").textContent = t("schemaColumnPassed");
      $("schema-column-state").dataset.kind = "success";
    } else {
      $("schema-column-state").textContent = t("schemaColumnFailed");
      $("schema-column-state").dataset.kind = "error";
    }
  } catch (error) {
    $("schema-column-state").textContent = error.message;
    $("schema-column-state").dataset.kind = "error";
  }
}

async function applySchemaColumn() {
  if (!schemaColumnPreview) return;
  const message = t("schemaColumnConfirm", schemaColumnPreview.column);
  if (!window.confirm(message)) return;
  try {
    const result = await api("schema-column-apply", {
      method: "POST",
      body: JSON.stringify({ ...schemaColumnPreview, confirm: true }),
    });
    const tab = tabs.get(result.path);
    if (tab) {
      tab.saved = result.content;
      tab.hash = result.hash;
      tab.state = EditorState.create({ doc: result.content });
      if (activePath === result.path && editor) { editor.setState(tab.state); tab.state = editor.state; }
      renderTabs();
    } else await openFile(result.path);
    resetSchemaColumnPreview();
    await refreshTree();
    await loadDatabaseSchema();
    announce(t("schemaColumnApplied"), "success");
  } catch (error) {
    $("schema-column-state").textContent = error.message;
    $("schema-column-state").dataset.kind = "error";
    announce(error.message, "error");
  }
}

function resetDatabaseRecordPreview() {
  databaseRecordRequest += 1;
  databaseRecordBusy = false;
  databaseRecordOffset = 0;
  databaseRecordOffsets = [0];
  const wrap = $("database-record-table-wrap");
  wrap.replaceChildren();
  wrap.hidden = true;
  $("database-record-page").textContent = "";
  $("database-record-status").textContent = t("recordPreviewIdle");
  $("database-record-status").dataset.kind = "";
  $("database-record-previous").disabled = true;
  $("database-record-next").disabled = true;
}

function renderDatabaseRecords(result) {
  const wrap = $("database-record-table-wrap");
  const columns = Array.isArray(result.columns) ? result.columns : [];
  const rows = Array.isArray(result.rows) ? result.rows : [];
  if (result.readOnly !== true) throw new Error("Studio received an unexpected writable data response");
  wrap.replaceChildren();
  if (!rows.length || !columns.length) {
    wrap.hidden = true;
    $("database-record-status").textContent = t("recordPreviewEmpty");
  } else {
    const table = document.createElement("table");
    table.className = "database-record-table";
    const heading = table.createTHead().insertRow();
    for (const name of columns) {
      const cell = document.createElement("th");
      cell.scope = "col";
      cell.textContent = String(name);
      heading.append(cell);
    }
    const body = table.createTBody();
    for (const values of rows) {
      const row = body.insertRow();
      for (let index = 0; index < columns.length; index += 1) {
        const cell = row.insertCell();
        cell.textContent = String(values[index] ?? "");
        cell.title = cell.textContent;
      }
    }
    wrap.append(table);
    wrap.hidden = false;
    $("database-record-status").textContent = result.truncated ? t("recordPreviewIncomplete") : "";
    $("database-record-status").dataset.kind = result.truncated ? "warning" : "";
  }
  $("database-record-page").textContent = rows.length
    ? t("recordPreviewPage", `${databaseRecordOffset + 1}–${databaseRecordOffset + rows.length}`)
    : "";
  $("database-record-previous").disabled = databaseRecordOffsets.length <= 1;
  $("database-record-next").disabled = !Number.isInteger(result.nextOffset);
}

function chartIsBinary(column) {
  return /\b(?:BINARY|VARBINARY|BLOB|BYTEA|IMAGE)\b/i.test(String(column.sql_type || column.type || ""));
}

function chartIsNumeric(column) {
  return /^(?:TINYINT|SMALLINT|MEDIUMINT|INT|INTEGER|BIGINT|DECIMAL|NUMERIC|FLOAT|DOUBLE|REAL)\b/i.test(String(column.sql_type || column.type || "").trim());
}

function chartIsText(column) {
  return /^(?:CHAR|VARCHAR|TEXT|TINYTEXT|MEDIUMTEXT|LONGTEXT|ENUM|SET|UUID|CHARACTER)\b/i.test(String(column.sql_type || column.type || "").trim());
}

function chartIsDate(column) {
  return /^DATE\b/i.test(String(column.sql_type || column.type || "").trim());
}

function configureChartBuilder(table) {
  const builder = $("chart-builder");
  const columns = (table?.live_columns || []).filter((column) => !chartIsBinary(column));
  builder.hidden = !table || !columns.length || table.status === "declared_missing_live";
  chartPreviewState = null;
  $("chart-preview").hidden = true;
  $("chart-save-button").disabled = true;
  $("presentation-add-button").disabled = true;
  $("chart-state").textContent = "";
  const group = $("chart-group");
  const measure = $("chart-measure");
  const filterColumn = $("chart-filter-column");
  const periodColumn = $("chart-period-column");
  group.replaceChildren();
  measure.replaceChildren();
  filterColumn.replaceChildren();
  periodColumn.replaceChildren();
  const noFilter = document.createElement("option");
  noFilter.value = "";
  noFilter.textContent = t("chartNoFilter");
  filterColumn.append(noFilter);
  const noPeriod = document.createElement("option");
  noPeriod.value = "";
  noPeriod.textContent = t("chartNoFilter");
  periodColumn.append(noPeriod);
  for (const column of columns) {
    const option = document.createElement("option");
    option.value = column.name;
    option.textContent = column.name;
    group.append(option);
  }
  for (const column of columns.filter(chartIsNumeric)) {
    const option = document.createElement("option");
    option.value = column.name;
    option.textContent = column.name;
    measure.append(option);
  }
  for (const column of columns.filter(chartIsText)) {
    const option = document.createElement("option");
    option.value = column.name;
    option.textContent = column.name;
    filterColumn.append(option);
  }
  for (const column of columns.filter(chartIsDate)) {
    const option = document.createElement("option");
    option.value = column.name;
    option.textContent = column.name;
    periodColumn.append(option);
  }
  $("chart-filter-value").value = "";
  $("chart-period-from").value = "";
  $("chart-period-to").value = "";
  if (!$("chart-name").value.trim()) $("chart-name").value = t("chartDefaultTitle", table?.name || "");
  updateChartMeasureAvailability();
}

function updateChartMeasureAvailability() {
  const aggregation = $("chart-aggregation").value;
  const count = aggregation === "count";
  $("chart-measure").disabled = count;
  $("chart-measure-wrap").hidden = count;
  for (const option of $("chart-aggregation").options) {
    if (["sum", "avg", "min", "max"].includes(option.value)) option.disabled = ![...$("chart-measure").options].some((item) => item.value);
  }
  if (!count && !$("chart-measure").value) {
    $("chart-state").textContent = t("chartPreviewFailed");
    $("chart-state").dataset.kind = "error";
  }
}

function invalidateChartPreview() {
  chartPreviewState = null;
  $("chart-save-button").disabled = true;
  $("presentation-add-button").disabled = true;
  if ($("chart-preview").hidden) return;
  $("chart-state").textContent = t("chartSaveFirst");
  $("chart-state").dataset.kind = "warning";
}

function currentChartSpec() {
  if (!selectedDatabaseTable) throw new Error(t("chartPreviewFailed"));
  const aggregation = $("chart-aggregation").value;
  const filterColumn = $("chart-filter-column").value;
  const filterValue = $("chart-filter-value").value;
  const periodColumn = $("chart-period-column").value;
  const periodFrom = $("chart-period-from").value;
  const periodTo = $("chart-period-to").value;
  return {
    version: 1,
    title: $("chart-name").value.trim() || t("chartDefaultTitle", selectedDatabaseTable.name),
    table: selectedDatabaseTable.name,
    groupBy: $("chart-group").value,
    aggregation,
    measure: aggregation === "count" ? null : $("chart-measure").value,
    kind: $("chart-kind").value,
    filter: filterColumn && filterValue ? { column: filterColumn, value: filterValue } : null,
    dateRange: periodColumn && (periodFrom || periodTo) ? { column: periodColumn, from: periodFrom || null, to: periodTo || null } : null,
  };
}

function svgElement(doc, name, attributes = {}, text = null) {
  const element = doc.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, String(value));
  if (text !== null) element.textContent = text;
  return element;
}

function chartNumber(value) {
  if (value === null || value === undefined || value === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function buildChartDisplay(spec, result, doc = document) {
  if (result.readOnly !== true || !Array.isArray(result.rows) || result.rows.length > 20) {
    throw new Error(t("chartPreviewFailed"));
  }
  const svg = doc.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 900 320");
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", t("chartAccessible", spec.title, spec.groupBy));
  svg.classList.add("chart-visualization");
  svg.append(svgElement(doc, "title", {}, spec.title));
  const rows = result.rows.map((row) => ({ label: String(row[0] ?? "—"), value: chartNumber(row[1]) }));
  const table = doc.createElement("table");
  table.className = "chart-data-table";
  const head = table.createTHead().insertRow();
  for (const label of [t("chartGroupLabel"), t("chartValueLabel")]) {
    const cell = doc.createElement("th"); cell.scope = "col"; cell.textContent = label; head.append(cell);
  }
  const body = table.createTBody();
  for (const row of rows) {
    const tr = body.insertRow();
    const label = tr.insertCell(); label.textContent = row.label;
    const value = tr.insertCell(); value.textContent = row.value === null ? t("chartInvalidNumber") : new Intl.NumberFormat(language, { maximumFractionDigits: 2 }).format(row.value);
  }
  if (!rows.length) {
    svg.append(svgElement(doc, "text", { x: 450, y: 160, "text-anchor": "middle", class: "chart-empty-label" }, t("chartEmpty")));
    return { svg, table };
  }
  if (rows.some((row) => row.value === null)) throw new Error(t("chartInvalidNumber"));
  const left = 64; const right = 24; const top = 32; const bottom = 62; const chartWidth = 900 - left - right; const chartHeight = 320 - top - bottom;
  const values = rows.map((row) => row.value);
  let minimum = Math.min(0, ...values); let maximum = Math.max(0, ...values);
  if (minimum === maximum) maximum = minimum + 1;
  const y = (value) => top + ((maximum - value) / (maximum - minimum)) * chartHeight;
  const baseline = y(0);
  svg.append(svgElement(doc, "line", { x1: left, y1: baseline, x2: 900 - right, y2: baseline, class: "chart-axis" }));
  const ticks = 4;
  for (let index = 0; index <= ticks; index += 1) {
    const value = maximum - ((maximum - minimum) * index) / ticks;
    const lineY = y(value);
    svg.append(svgElement(doc, "line", { x1: left, y1: lineY, x2: 900 - right, y2: lineY, class: "chart-grid-line" }));
    svg.append(svgElement(doc, "text", { x: left - 9, y: lineY + 4, "text-anchor": "end", class: "chart-tick" }, new Intl.NumberFormat(language, { maximumFractionDigits: 1 }).format(value)));
  }
  const slot = chartWidth / rows.length;
  if (spec.kind === "line") {
    const points = rows.map((row, index) => `${left + slot * (index + 0.5)},${y(row.value)}`).join(" ");
    svg.append(svgElement(doc, "polyline", { points, class: "chart-line" }));
    rows.forEach((row, index) => svg.append(svgElement(doc, "circle", { cx: left + slot * (index + 0.5), cy: y(row.value), r: 4, class: "chart-point" })));
  } else {
    rows.forEach((row, index) => {
      const barWidth = Math.min(64, slot * 0.62);
      const x = left + slot * index + (slot - barWidth) / 2;
      const valueY = y(row.value);
      svg.append(svgElement(doc, "rect", { x, y: Math.min(valueY, baseline), width: barWidth, height: Math.max(1, Math.abs(baseline - valueY)), rx: 4, class: "chart-bar" }));
    });
  }
  rows.forEach((row, index) => {
    const x = left + slot * (index + 0.5);
    const label = row.label.length > 16 ? `${row.label.slice(0, 15)}…` : row.label;
    svg.append(svgElement(doc, "text", { x, y: 320 - 31, "text-anchor": "middle", class: "chart-label" }, label));
  });
  return { svg, table };
}

function renderChartPreview(spec, result) {
  const display = buildChartDisplay(spec, result);
  display.svg.id = "chart-svg";
  $("chart-svg").replaceWith(display.svg);
  const tableWrap = $("chart-data-table-wrap");
  tableWrap.replaceChildren(display.table);
  $("chart-preview").hidden = false;
  $("chart-truncated").hidden = result.truncated !== true;
}

async function previewChart() {
  const state = $("chart-state");
  state.textContent = t("chartPreviewRunning"); state.dataset.kind = "pending";
  $("chart-preview-button").disabled = true;
  $("chart-save-button").disabled = true;
  $("presentation-add-button").disabled = true;
  try {
    const spec = currentChartSpec();
    const result = await api("database-chart-preview", { method: "POST", body: JSON.stringify(spec) });
    renderChartPreview(spec, result);
    chartPreviewState = { spec, result };
    $("chart-save-button").disabled = false;
    $("presentation-add-button").disabled = false;
    state.textContent = t("chartPreviewReady"); state.dataset.kind = "success";
  } catch (error) {
    chartPreviewState = null;
    state.textContent = `${t("chartPreviewFailed")} ${error.message}`; state.dataset.kind = "error";
    announce(state.textContent, "error");
  } finally {
    $("chart-preview-button").disabled = false;
  }
}

function fileSlug(value) {
  const slug = String(value).normalize("NFKD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 48);
  return slug || "report";
}

async function saveStudioJson(path, value) {
  const content = `${JSON.stringify(value, null, 2)}\n`;
  if (treeFiles.includes(path)) {
    const current = await api(`file?path=${encodeURIComponent(path)}`);
    let existing;
    try { existing = JSON.parse(current.content); } catch { throw new Error(t("reportFileConflict")); }
    if (existing?.schema !== value.schema) throw new Error(t("reportFileConflict"));
    await api("file", { method: "PUT", body: JSON.stringify({ path, content, baseHash: current.hash }) });
  } else {
    await api("file", { method: "POST", body: JSON.stringify({ path, content }) });
  }
  await refreshTree();
  return path;
}

async function saveChart() {
  if (!chartPreviewState) { announce(t("chartSaveFirst"), "warning"); return; }
  const path = `chart-${fileSlug(chartPreviewState.spec.title)}.zchart.json`;
  try {
    await saveStudioJson(path, { schema: "zelyra.chart.v1", ...chartPreviewState.spec });
    $("chart-state").textContent = t("chartSaved", path); $("chart-state").dataset.kind = "success";
    announce(t("chartSaved", path), "success");
  } catch (error) {
    $("chart-state").textContent = `${t("chartSaveFailed")} ${error.message}`; $("chart-state").dataset.kind = "error";
    announce($("chart-state").textContent, "error");
  }
}

function renderSavedReportChoices() {
  for (const [selectId, suffix, buttonId] of [
    ["saved-chart-select", ".zchart.json", "saved-chart-add"],
    ["saved-presentation-select", ".zelyra-presentation.json", "saved-presentation-load"],
  ]) {
    const select = $(selectId);
    if (!select) continue;
    const selected = select.value;
    const files = treeFiles.filter((path) => path.endsWith(suffix));
    select.replaceChildren();
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = files.length ? t("chooseFile") : t("reportFilesEmpty");
    select.append(placeholder);
    for (const path of files) {
      const option = document.createElement("option");
      option.value = path;
      option.textContent = path;
      select.append(option);
    }
    if (files.includes(selected)) select.value = selected;
    $(buttonId).disabled = files.length === 0 || !select.value;
  }
}

function normalizeSavedChart(value, requireSchema = false) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(t("chartInvalidConfig"));
  if (requireSchema && value.schema !== "zelyra.chart.v1") throw new Error(t("chartInvalidConfig"));
  if (value.version !== 1 || typeof value.title !== "string" || !value.title.trim() || value.title.length > 80) throw new Error(t("chartInvalidConfig"));
  if (typeof value.table !== "string" || !value.table || value.table.length > 128) throw new Error(t("chartInvalidConfig"));
  if (typeof value.groupBy !== "string" || !value.groupBy || value.groupBy.length > 128) throw new Error(t("chartInvalidConfig"));
  if (!["count", "sum", "avg", "min", "max"].includes(value.aggregation)) throw new Error(t("chartInvalidConfig"));
  if (value.aggregation === "count" ? value.measure !== null : (typeof value.measure !== "string" || !value.measure || value.measure.length > 128)) throw new Error(t("chartInvalidConfig"));
  if (!["bar", "line"].includes(value.kind)) throw new Error(t("chartInvalidConfig"));
  let filter = null;
  if (value.filter !== null && value.filter !== undefined) {
    if (!value.filter || typeof value.filter.column !== "string" || !value.filter.column || value.filter.column.length > 128 ||
        typeof value.filter.value !== "string" || value.filter.value.length > 256) throw new Error(t("chartInvalidConfig"));
    filter = { column: value.filter.column, value: value.filter.value };
  }
  let dateRange = null;
  if (value.dateRange !== null && value.dateRange !== undefined) {
    const from = value.dateRange?.from || null;
    const to = value.dateRange?.to || null;
    if (!value.dateRange || typeof value.dateRange.column !== "string" || !value.dateRange.column || value.dateRange.column.length > 128 ||
        (from !== null && !/^\d{4}-\d{2}-\d{2}$/.test(from)) || (to !== null && !/^\d{4}-\d{2}-\d{2}$/.test(to))) throw new Error(t("chartInvalidConfig"));
    dateRange = { column: value.dateRange.column, from, to };
  }
  return {
    version: 1,
    title: value.title,
    table: value.table,
    groupBy: value.groupBy,
    aggregation: value.aggregation,
    measure: value.measure,
    kind: value.kind,
    filter,
    dateRange,
  };
}

async function readStudioJson(path) {
  const response = await api("file?path=" + encodeURIComponent(path));
  try { return JSON.parse(response.content); }
  catch { throw new Error(t("chartInvalidConfig")); }
}

async function loadSavedChart() {
  const path = $("saved-chart-select").value;
  if (!path) return;
  try {
    const spec = normalizeSavedChart(await readStudioJson(path), true);
    const table = databaseTables.find((candidate) => candidate.name === spec.table && candidate.status !== "live_unmapped");
    if (!table) throw new Error(t("chartInvalidConfig"));
    selectDatabaseTable(table.name);
    $("chart-name").value = spec.title;
    $("chart-group").value = spec.groupBy;
    $("chart-aggregation").value = spec.aggregation;
    updateChartMeasureAvailability();
    if (spec.measure !== null) $("chart-measure").value = spec.measure;
    $("chart-kind").value = spec.kind;
    $("chart-filter-column").value = spec.filter?.column || "";
    $("chart-filter-value").value = spec.filter?.value || "";
    $("chart-period-column").value = spec.dateRange?.column || "";
    $("chart-period-from").value = spec.dateRange?.from || "";
    $("chart-period-to").value = spec.dateRange?.to || "";
    await previewChart();
    if (!chartPreviewState) return;
    addChartToPresentation();
    announce(t("chartAdded"), "success");
  } catch (error) {
    announce(t("chartInvalidConfig") + " " + error.message, "error");
  }
}

async function loadSavedPresentation() {
  const path = $("saved-presentation-select").value;
  if (!path) return;
  try {
    const value = await readStudioJson(path);
    if (!value || value.schema !== "zelyra.presentation.v1" || typeof value.title !== "string" || value.title.length > 100 ||
        typeof value.introduction !== "string" || value.introduction.length > 1000 ||
        !Array.isArray(value.charts) || value.charts.length < 1 || value.charts.length > 24) {
      throw new Error(t("presentationInvalidConfig"));
    }
    const charts = value.charts.map((chart) => normalizeSavedChart(chart));
    $("presentation-name").value = value.title;
    $("presentation-note").value = value.introduction;
    presentationCharts = charts;
    renderPresentationCharts();
    announce(t("presentationLoaded", path), "success");
  } catch (error) {
    announce(t("presentationInvalidConfig") + " " + error.message, "error");
  }
}

function renderPresentationCharts() {
  const count = $("presentation-count");
  count.textContent = presentationCharts.length
    ? t("presentationCount", presentationCharts.length).replace("{1}", $("presentation-name").value.trim())
    : t("presentationEmpty");
  $("presentation-save-button").disabled = !presentationCharts.length;
  $("presentation-print-button").disabled = !presentationCharts.length;
  const old = $("presentation-slide-list");
  if (old) old.remove();
  if (!presentationCharts.length) return;
  const list = document.createElement("div"); list.id = "presentation-slide-list"; list.className = "presentation-slides";
  presentationCharts.forEach((spec, index) => {
    const chip = document.createElement("span"); chip.className = "presentation-slide-chip";
    const label = document.createElement("span"); label.textContent = `${index + 1}. ${spec.title}`;
    const remove = document.createElement("button"); remove.type = "button"; remove.textContent = "×"; remove.title = t("presentationRemove"); remove.setAttribute("aria-label", `${t("presentationRemove")} ${spec.title}`);
    remove.addEventListener("click", () => { presentationCharts.splice(index, 1); renderPresentationCharts(); });
    chip.append(label, remove); list.append(chip);
  });
  $("presentation-count").parentElement.after(list);
}

function addChartToPresentation() {
  if (!chartPreviewState) { announce(t("chartSaveFirst"), "warning"); return; }
  const signature = JSON.stringify(chartPreviewState.spec);
  if (!presentationCharts.some((item) => JSON.stringify(item) === signature)) presentationCharts.push(chartPreviewState.spec);
  renderPresentationCharts();
}

function presentationPayload() {
  return {
    schema: "zelyra.presentation.v1",
    title: $("presentation-name").value.trim() || t("presentationTitle"),
    introduction: $("presentation-note").value.trim(),
    charts: presentationCharts,
  };
}

async function savePresentation() {
  if (!presentationCharts.length) { announce(t("presentationNeedCharts"), "warning"); return; }
  const payload = presentationPayload();
  const path = `presentation-${fileSlug(payload.title)}.zelyra-presentation.json`;
  try {
    await saveStudioJson(path, payload);
    announce(t("presentationSaved", path), "success");
  } catch (error) {
    announce(`${t("presentationSaveFailed", error.message)}`, "error");
  }
}

async function printPresentation() {
  if (!presentationCharts.length) { announce(t("presentationNeedCharts"), "warning"); return; }
  const win = window.open("", "_blank");
  if (!win) { announce(t("presentationPrintFailed"), "error"); return; }
  const doc = win.document;
  doc.open();
  doc.write("<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Zelyra report</title><style>body{margin:0;padding:30px;background:#edf0f5;color:#172032;font:14px system-ui,sans-serif}.deck{max-width:980px;margin:auto}.chart-print-page{min-height:680px;margin:0 0 24px;padding:52px 58px;background:#fff;box-shadow:0 8px 40px #1b274022;page-break-after:always}.chart-print-page h1{font-size:30px;margin:0 0 8px}.chart-print-page h2{font-size:22px;margin:22px 0 4px}.chart-print-page p{color:#506076;line-height:1.55}.chart-print-kicker{color:#6557d9;font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase}.chart-print-page svg{width:100%;height:auto;max-height:420px;margin:22px 0}.chart-print-page table{width:100%;border-collapse:collapse}.chart-print-page th,.chart-print-page td{padding:8px;border-bottom:1px solid #dbe2ee;text-align:left}.chart-print-footer{margin-top:28px;color:#7a8596;font-size:9px}@media print{body{padding:0;background:#fff}.chart-print-page{min-height:0;margin:0;padding:16mm 17mm;box-shadow:none}.chart-print-page:last-child{page-break-after:auto}}</style></head><body><main class=\"deck\"></main></body></html>");
  doc.close();
  const chartStyles = doc.createElement("style");
  chartStyles.textContent = ".chart-axis{stroke:#506076;stroke-width:1}.chart-grid-line{stroke:#dbe2ee;stroke-width:1}.chart-tick,.chart-label{fill:#506076;font:11px ui-monospace,monospace}.chart-bar{fill:#6557d9;opacity:.88}.chart-line{fill:none;stroke:#6557d9;stroke-width:3;stroke-linecap:round;stroke-linejoin:round}.chart-point{fill:#6557d9;stroke:#fff;stroke-width:2}.chart-empty-label{fill:#506076;font:13px system-ui,sans-serif}";
  doc.head.append(chartStyles);
  const payload = presentationPayload();
  doc.title = payload.title;
  const deck = doc.querySelector(".deck");
  const cover = doc.createElement("section"); cover.className = "chart-print-page";
  const kicker = doc.createElement("p"); kicker.className = "chart-print-kicker"; kicker.textContent = t("presentationKicker");
  const heading = doc.createElement("h1"); heading.textContent = payload.title;
  const note = doc.createElement("p"); note.textContent = payload.introduction;
  cover.append(kicker, heading, note); deck.append(cover);
  try {
    for (const [index, spec] of presentationCharts.entries()) {
      const result = await api("database-chart-preview", { method: "POST", body: JSON.stringify(spec) });
      const display = buildChartDisplay(spec, result, doc);
      const slide = doc.createElement("section"); slide.className = "chart-print-page";
      const slideKicker = doc.createElement("p"); slideKicker.className = "chart-print-kicker"; slideKicker.textContent = t("presentationNumber", `${index + 1} / ${presentationCharts.length}`);
      const title = doc.createElement("h2"); title.textContent = spec.title;
      const source = doc.createElement("p"); source.textContent = `${spec.table} · ${spec.groupBy} · ${spec.aggregation}${spec.measure ? ` (${spec.measure})` : ""}`;
      const footer = doc.createElement("p"); footer.className = "chart-print-footer"; footer.textContent = t("presentationFooter");
      slide.append(slideKicker, title, source, display.svg, display.table, footer); deck.append(slide);
    }
    win.setTimeout(() => win.print(), 400);
  } catch (error) {
    win.close(); announce(`${t("presentationLoadFailed")} ${error.message}`, "error");
  }
}

async function loadDatabaseRecords(offset = 0) {
  if (!selectedDatabaseTable || databaseRecordBusy) return;
  const table = selectedDatabaseTable.name;
  const requestId = ++databaseRecordRequest;
  databaseRecordOffset = offset;
  databaseRecordBusy = true;
  $("database-record-load").disabled = true;
  $("database-record-previous").disabled = true;
  $("database-record-next").disabled = true;
  $("database-record-status").textContent = t("recordPreviewLoading");
  $("database-record-status").dataset.kind = "pending";
  try {
    const result = await api(`database-rows?table=${encodeURIComponent(table)}&limit=25&offset=${offset}`);
    if (requestId !== databaseRecordRequest || selectedDatabaseTable?.name !== table) return;
    renderDatabaseRecords(result);
    if (offset === 0) databaseRecordOffsets = [0];
    else if (databaseRecordOffsets.at(-1) !== offset) databaseRecordOffsets.push(offset);
  } catch (error) {
    if (requestId !== databaseRecordRequest) return;
    $("database-record-table-wrap").replaceChildren();
    $("database-record-table-wrap").hidden = true;
    $("database-record-status").textContent = `${t("recordPreviewFailed")} ${error.message}`;
    $("database-record-status").dataset.kind = "error";
  } finally {
    if (requestId === databaseRecordRequest) {
      databaseRecordBusy = false;
      $("database-record-load").disabled = false;
      $("database-record-previous").disabled = databaseRecordOffsets.length <= 1;
    }
  }
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
  void loadDatabaseTarget();
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
      const result = await api("check", { method: "POST", body: JSON.stringify({
        ...analysisRequest(path, source, sequence), clientId: eventClientId,
      }) });
      if (sequence !== liveCheckSequence || activePath !== path || editor?.state.doc.toString() !== source) return;
      renderDiagnostics(result.diagnostics || [], result.success, { silent: true });
      void refreshPipelineContext(path, source, sequence);
      if (!result.success) return;
      const verified = await api("verify", { method: "POST", body: JSON.stringify({
        ...analysisRequest(path, source, sequence), clientId: eventClientId,
      }) });
      if (sequence !== liveCheckSequence || activePath !== path || editor?.state.doc.toString() !== source || verified.version !== sequence) return;
      setVerificationResults(verified.success ? verified.results || [] : []);
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

function studioEntryPath() {
  return treeFiles.includes("main.zyl")
    ? "main.zyl"
    : (activePath && activePath.endsWith(".zyl") ? activePath : (treeFiles.find(f => f.endsWith(".zyl")) || "main.zyl"));
}

async function openRuntimeBudget() {
  const dialog = $("runtime-budget-dialog");
  if (!dialog.open) dialog.showModal();
  const note = $("runtime-profile-note");
  const risks = $("runtime-recursion-risks");
  note.textContent = t("checkRunning");
  risks.replaceChildren();
  try {
    const path = studioEntryPath();
    const source = path === activePath ? editor?.state.doc.toString() : undefined;
    const result = await api("runtime-profile", { method: "POST", body: JSON.stringify({
      ...analysisRequest(path, source),
    }) });
    if (!dialog.open) return;
    $("runtime-stack-depth").textContent = t("runtimeStackDepth", result.maxStackDepth,
      t(result.configured ? "runtimeStackConfigured" : "runtimeStackDefault"));
    note.textContent = result.recursionAnalysisIncomplete ? t("runtimeRecursionIncomplete")
      : result.success ? "" : t("runtimeProfileInvalid");
    for (const risk of result.recursionRisks || []) {
      const item = document.createElement("li");
      const names = (risk.functions || []).map((fn) => `${fn.name} (${fn.path}:${fn.line})`).join(" → ");
      item.textContent = `${t(risk.kind === "mutual" ? "runtimeRecursionMutual" : "runtimeRecursionDirect")}: ${names}`;
      risks.append(item);
    }
    if (!risks.childElementCount && !result.recursionAnalysisIncomplete && result.success) {
      const item = document.createElement("li");
      item.textContent = t("runtimeRecursionNone");
      risks.append(item);
    }
  } catch (error) {
    note.textContent = t("runtimeProfileError", error.message);
  }
}

async function runProject() {
  let maxSteps = null;
  if ($("runtime-budget-enabled").checked) {
    maxSteps = Number($("runtime-budget-number").value);
    if (!Number.isSafeInteger(maxSteps) || maxSteps < 1 || maxSteps > 100_000_000) {
      announce(t("runtimeBudgetInvalid"), "error");
      if (!$("runtime-budget-dialog").open) $("runtime-budget-dialog").showModal();
      $("runtime-budget-number").focus();
      return;
    }
  }
  if (!(await saveAll())) return;
  const dialog = $("run-output");
  const state = $("run-output-state");
  const output = $("run-output-content");
  state.textContent = t("runStarting");
  state.dataset.kind = "pending";
  output.textContent = "";
  $("run-output-web-link").hidden = true;
  $("run-output-web-url").removeAttribute("href");
  if (!dialog.open) dialog.showModal();
  $("run-output-cancel").hidden = false;
  $("run-output-cancel").disabled = false;
  announce(t("runStarting"));
  try {
    const entryPath = studioEntryPath();
    const result = await api("run", { method: "POST", body: JSON.stringify({ path: entryPath, maxSteps }) });
    const webLink = $("run-output-web-link");
    if (result.webApp && result.webUrl) {
      state.textContent = t("runWebReady");
      state.dataset.kind = "success";
      webLink.hidden = false;
      $("run-output-web-url").href = result.webUrl;
      output.textContent = result.webUrl;
      announce(`${t("runWebReady")} ${result.webUrl}`, "success");
      return;
    }
    webLink.hidden = true;
    const statusText = result.cancelled ? t("runCancelled") : result.timedOut ? t("runTimeout")
      : result.budgetExceeded ? t("runtimeBudgetExceeded") : result.success ? t("runSuccess") : t("runFailure");
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
    output.textContent = error.message;
    announce(error.message, "error");
  } finally {
    $("run-output-cancel").hidden = true;
  }
}

async function stopWebPreview() {
  try {
    await api("stop-web-preview", { method: "POST", body: "{}" });
    $("run-output-web-link").hidden = true;
    $("run-output-state").textContent = t("runWebStopped");
    $("run-output-state").dataset.kind = "success";
    $("run-output-web-url").removeAttribute("href");
    announce(t("runWebStopped"), "success");
  } catch (error) {
    $("run-output-state").textContent = error.message;
    $("run-output-state").dataset.kind = "error";
    announce(error.message, "error");
  }
}

function renderEditorLogs() {
  const level = $("logs-level").value;
  const query = $("logs-search").value.trim().toLocaleLowerCase();
  const records = editorLogRecords.filter((record) => {
    if (level !== "all" && record.level !== level) return false;
    return !query || `${record.source || ""} ${record.message || ""}`.toLocaleLowerCase().includes(query);
  });
  const locale = language === "de" ? "de-DE" : "en-US";
  const lines = records.map((record) => {
    const timestamp = new Date(Number(record.timestamp_ms)).toLocaleString(locale);
    return `${timestamp}  ${(record.level || "info").toUpperCase()}  ${record.source || "zelyra"}\n${record.message || ""}`;
  });
  $("logs-content").textContent = lines.join("\n\n") || t("logsEmpty");
  $("logs-status").textContent = t("logsCount", records.length);
}

async function openEditorLogs() {
  const dialog = $("logs-dialog");
  if (!dialog.open) dialog.showModal();
  $("logs-status").textContent = t("logsLoading");
  try {
    const result = await api("logs");
    editorLogRecords = Array.isArray(result.records) ? result.records : [];
    renderEditorLogs();
  } catch {
    $("logs-status").textContent = t("logsUnavailable");
    $("logs-content").textContent = t("logsUnavailable");
  }
}

function renderPipelineBreadcrumbs() {
  const target = $("pipeline-breadcrumbs");
  target.replaceChildren();
  if (!editor || !pipelineMapCache || pipelineMapCache.path !== activePath) return;
  const map = pipelineMapCache.map;
  const cursor = editor.state.selection.main.head;
  const local = (map.nodes || []).filter((node) => node.path === activePath && node.kind !== "module"
    && typeof node.position === "number" && node.position <= cursor);
  local.sort((left, right) => right.position - left.position);
  const selected = local[0];
  if (!selected) return;
  const nodes = new Map((map.nodes || []).map((node) => [node.id, node]));
  const shown = [selected];
  const visited = new Set([selected.id]);
  let frontier = [selected.id];
  for (let depth = 0; depth < 2 && shown.length < 8; depth += 1) {
    const next = [];
    for (const id of frontier) {
      for (const edge of map.edges || []) {
        if (edge.label === "declares" || (edge.from !== id && edge.to !== id)) continue;
        const neighborId = edge.from === id ? edge.to : edge.from;
        const neighbor = nodes.get(neighborId);
        if (!neighbor || neighbor.kind === "module" || visited.has(neighborId)) continue;
        visited.add(neighborId);
        shown.push(neighbor);
        next.push(neighborId);
        if (shown.length >= 8) break;
      }
      if (shown.length >= 8) break;
    }
    frontier = next;
  }
  for (const node of shown) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = node.id === selected.id ? "current" : "";
    button.textContent = `${node.kind}: ${node.label}`;
    button.title = `${node.path}:${node.line || 1}`;
    button.addEventListener("click", () => {
      void openFile(node.path).then(() => {
        if (activePath !== node.path || !editor) return;
        editor.dispatch({ selection: { anchor: Math.min(node.position || 0, editor.state.doc.length) }, scrollIntoView: true });
        editor.focus();
      });
    });
    target.append(button);
  }
}

async function refreshPipelineContext(path, source, version) {
  try {
    const map = await api("project-map", { method: "POST", body: JSON.stringify(analysisRequest(path, source, version)) });
    if (activePath !== path || editor?.state.doc.toString() !== source || liveCheckSequence !== version) return;
    pipelineMapCache = { path, map };
    renderPipelineBreadcrumbs();
  } catch {
    if (activePath === path && liveCheckSequence === version) {
      pipelineMapCache = null;
      renderPipelineBreadcrumbs();
    }
  }
}

async function openProjectMap() {
  const dialog = $("project-map-dialog");
  const status = $("project-map-status");
  const graph = $("project-map-graph");
  if (!dialog.open) dialog.showModal();
  status.textContent = t("projectMapLoading");
  graph.replaceChildren();
  $("project-map-nullability").hidden = true;
  $("project-map-nullability-list").replaceChildren();
  try {
    const path = activePath?.endsWith(".zyl") ? activePath : null;
    const source = path && editor ? editor.state.doc.toString() : null;
    const version = liveCheckSequence;
    const map = await api("project-map", { method: "POST", body: JSON.stringify(analysisRequest(path, source, version)) });
    if (!dialog.open || (source !== null && (path !== activePath || editor?.state.doc.toString() !== source))) return;
    renderProjectMap(map);
    const issueLabels = { ...messages[language].projectMapIssues, ...messages[language].projectMapIssuesExtra };
    const issues = (map.issues || []).map((code) => issueLabels[code] || code);
    status.textContent = map.incomplete ? `${t("projectMapIncomplete")} ${issues.join(" · ")}` : "";
    if (!map.nodes?.length) status.textContent = t("projectMapEmpty");
    const radar = $("project-map-nullability");
    const list = $("project-map-nullability-list");
    list.replaceChildren();
    for (const warning of map.nullabilityRadar || []) {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = `${warning.code} · ${warning.file}:${warning.span?.start?.line || 1} · ${warning.message}`;
      button.addEventListener("click", () => {
        void openFile(warning.file).then(() => {
          if (activePath !== warning.file || !editor) return;
          const offset = warning.editorSpan?.start ?? 0;
          editor.dispatch({ selection: { anchor: Math.min(offset, editor.state.doc.length) }, scrollIntoView: true });
          editor.focus();
          dialog.close();
        });
      });
      list.append(button);
    }
    radar.hidden = !list.childElementCount;
  } catch (error) {
    status.textContent = error.message || t("projectMapUnavailable");
  }
}

let pdfPreviewUrl = null;
function clearPdfPreview() {
  $("pdf-preview-frame").hidden = true;
  $("pdf-preview-frame").removeAttribute("src");
  $("pdf-preview-download").hidden = true;
  $("pdf-preview-download").removeAttribute("href");
  if (pdfPreviewUrl) URL.revokeObjectURL(pdfPreviewUrl);
  pdfPreviewUrl = null;
}

async function openPdfPreview() {
  const dialog = $("pdf-preview-dialog");
  const select = $("pdf-preview-view");
  const status = $("pdf-preview-status");
  if (!dialog.open) dialog.showModal();
  status.textContent = t("pdfLoading");
  clearPdfPreview();
  select.replaceChildren();
  try {
    const response = await api("pdf-views");
    if (!dialog.open) return;
    for (const name of response.views || []) {
      const option = document.createElement("option");
      option.value = name;
      option.textContent = name;
      select.append(option);
    }
    status.textContent = select.options.length ? "" : t("pdfNoViews");
    $("pdf-preview-render").disabled = !select.options.length;
    if (select.options.length) select.focus();
  } catch (error) {
    status.textContent = t("pdfError", error.message);
    $("pdf-preview-render").disabled = true;
  }
}

async function renderPdfPreview() {
  const status = $("pdf-preview-status");
  let record;
  try {
    record = JSON.parse($("pdf-preview-data").value);
    if (!record || Array.isArray(record) || typeof record !== "object") throw new Error();
  } catch {
    status.textContent = t("pdfInvalidData");
    return;
  }
  const view = $("pdf-preview-view").value;
  const path = activePath?.endsWith(".zyl") ? activePath : null;
  const source = path && editor ? editor.state.doc.toString() : null;
  clearPdfPreview();
  status.textContent = t("pdfRendering");
  $("pdf-preview-render").disabled = true;
  try {
    const result = await api("pdf-render", { method: "POST", body: JSON.stringify({
      view, record, ...(path && source !== null ? { path, source } : {}),
    }) });
    if (!$("pdf-preview-dialog").open || (source !== null && (path !== activePath || editor?.state.doc.toString() !== source))) return;
    const decoded = atob(result.base64);
    const bytes = new Uint8Array(decoded.length);
    for (let index = 0; index < decoded.length; index++) bytes[index] = decoded.charCodeAt(index);
    if (result.mimeType !== "application/pdf" || decoded.slice(0, 5) !== "%PDF-") throw new Error("Invalid PDF response");
    pdfPreviewUrl = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
    $("pdf-preview-frame").src = pdfPreviewUrl;
    $("pdf-preview-frame").hidden = false;
    $("pdf-preview-download").href = pdfPreviewUrl;
    $("pdf-preview-download").hidden = false;
    status.textContent = t("pdfReady", result.bytes);
  } catch (error) {
    status.textContent = t("pdfError", error.message);
  } finally {
    $("pdf-preview-render").disabled = false;
  }
}

let canvasEntries = [];
let canvasSequence = 0;
let canvasTimer = null;
let canvasDeclaration = null;
let canvasThemeSource = null;
let canvasColors = [];

function themeColorDeclarations(source) {
  const declarations = [];
  const blocks = [];
  for (let i = 0; i < source.length;) {
    if (source.startsWith("/*", i)) {
      const end = source.indexOf("*/", i + 2);
      i = end < 0 ? source.length : end + 2;
      continue;
    }
    if (source[i] === '"' || source[i] === "'") {
      const quote = source[i++];
      while (i < source.length && source[i] !== quote) i += source[i] === "\\" ? 2 : 1;
      i++;
      continue;
    }
    if (source[i] === "{") { blocks.push(i); i++; continue; }
    if (source[i] === "}") { blocks.pop(); i++; continue; }
    if (blocks.length && source.startsWith("--z-color-", i)
        && !/[\w-]/.test(source[i - 1] || "")) {
      const name = /^--z-color-[a-z0-9-]+/.exec(source.slice(i))?.[0];
      if (name) {
        let from = i + name.length;
        while (/\s/.test(source[from] || "")) from++;
        if (source[from] === ":") {
          from++;
          while (/\s/.test(source[from] || "")) from++;
          const color = /^#(?:[0-9a-f]{6}|[0-9a-f]{3})(?![0-9a-f])/i.exec(source.slice(from))?.[0];
          if (color) {
            declarations.push({ name, from, to: from + color.length, color,
              block: blocks[blocks.length - 1], line: source.slice(0, i).split("\n").length });
            i = from + color.length;
            continue;
          }
        }
      }
    }
    i++;
  }
  return declarations;
}

function normalizedColor(color) {
  return color.length === 4 ? `#${[...color.slice(1)].map((part) => part + part).join("")}` : color.toLowerCase();
}

function readableOnColor(background) {
  const color = normalizedColor(background);
  const channels = [1, 3, 5].map((index) => parseInt(color.slice(index, index + 2), 16) / 255)
    .map((part) => part <= 0.04045 ? part / 12.92 : ((part + 0.055) / 1.055) ** 2.4);
  const luminance = channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
  return luminance >= 0.179 ? "#000000" : "#ffffff";
}

async function loadThemeColors() {
  const select = $("canvas-theme-token");
  const previous = canvasColors[Number(select.value)];
  const previousKey = previous ? `${previous.name}:${previous.line}` : "";
  try {
    const tab = tabs.get("zelyra.theme.css");
    const source = tab ? tabState("zelyra.theme.css", tab).doc.toString()
      : (await api("file?path=zelyra.theme.css")).content;
    if ($("canvas-pane").hidden) return;
    canvasThemeSource = source;
    canvasColors = themeColorDeclarations(source);
    select.replaceChildren();
    canvasColors.forEach((entry, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = `${entry.name} · ${entry.line}`;
      select.append(option);
    });
    const index = canvasColors.findIndex((entry) => `${entry.name}:${entry.line}` === previousKey);
    select.value = String(index < 0 ? 0 : index);
    $("canvas-theme-color").value = normalizedColor(canvasColors[Number(select.value)]?.color || "#3157a4");
    $("canvas-theme-apply").disabled = !canvasColors.length;
    $("canvas-theme-status").textContent = canvasColors.length ? "" : t("canvasThemeNoHex");
  } catch {
    canvasThemeSource = null;
    canvasColors = [];
    select.replaceChildren();
    $("canvas-theme-apply").disabled = true;
    $("canvas-theme-status").textContent = t("canvasThemeMissing");
  }
}

async function applyThemeColor() {
  const selected = canvasColors[Number($("canvas-theme-token").value)];
  if (!selected || !canvasThemeSource) return;
  try {
    await openFile("zelyra.theme.css");
    if (activePath !== "zelyra.theme.css" || !editor) return;
    const source = editor.state.doc.toString();
    if (source !== canvasThemeSource) {
      $("canvas-theme-status").textContent = t("canvasThemeStale");
      await loadThemeColors();
      return;
    }
    const color = $("canvas-theme-color").value;
    const changes = [{ from: selected.from, to: selected.to, insert: color }];
    const group = selected.name.slice("--z-color-".length);
    if (["primary", "success", "warning", "danger", "info"].includes(group)) {
      const pairName = `--z-color-on-${group}`;
      const onColor = readableOnColor(color);
      const pair = canvasColors.find((entry) => entry.block === selected.block && entry.name === pairName);
      if (pair) changes.push({ from: pair.from, to: pair.to, insert: onColor });
      else {
        const next = /^\s*;/.exec(source.slice(selected.to));
        if (!next) throw new Error(`${pairName} needs a semicolon after ${selected.name}`);
        const afterSemicolon = selected.to + next[0].length;
        changes.push({ from: afterSemicolon, to: afterSemicolon, insert: `\n    ${pairName}: ${onColor};` });
      }
    }
    changes.sort((left, right) => left.from - right.from);
    editor.dispatch({ changes });
    await loadThemeColors();
    $("canvas-theme-status").textContent = t("canvasThemeChanged");
    scheduleCanvasRefresh();
  } catch (error) { $("canvas-theme-status").textContent = t("canvasThemeError", error.message); }
}

function canvasRequestSource() {
  const path = activePath?.endsWith(".zyl") ? activePath : undefined;
  const themeTab = tabs.get("zelyra.theme.css");
  return {
    ...analysisRequest(path, path ? editor?.state.doc.toString() : undefined),
    themeSource: themeTab ? tabState("zelyra.theme.css", themeTab).doc.toString() : undefined,
    version: liveCheckSequence,
  };
}

function scheduleCanvasRefresh() {
  if ($("canvas-pane").hidden) return;
  clearTimeout(canvasTimer);
  canvasTimer = setTimeout(() => {
    void loadCanvasEntries();
    if (activePath === "zelyra.theme.css") void loadThemeColors();
  }, 700);
}

async function loadCanvasEntries() {
  if ($("canvas-pane").hidden) return;
  const sequence = ++canvasSequence;
  const target = $("canvas-target");
  const previous = canvasEntries[Number(target.value)];
  const previousKey = previous ? `${previous.kind}:${previous.name}` : "";
  $("canvas-status").textContent = t("canvasLoading");
  try {
    const result = await api("canvas-list", { method: "POST", body: JSON.stringify(canvasRequestSource()) });
    if (sequence !== canvasSequence || $("canvas-pane").hidden) return;
    canvasEntries = result.entries || [];
    target.replaceChildren();
    canvasEntries.forEach((entry, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = `${entry.kind} · ${entry.name}`;
      target.append(option);
    });
    const index = canvasEntries.findIndex((entry) => `${entry.kind}:${entry.name}` === previousKey);
    target.value = String(index < 0 ? 0 : index);
    if (!canvasEntries.length) {
      $("canvas-status").textContent = result.success ? t("canvasEmpty") : t("canvasIncomplete");
      $("canvas-frame").srcdoc = "";
      $("canvas-source").disabled = true;
      return;
    }
    await renderCanvas(sequence);
  } catch (error) {
    if (sequence === canvasSequence && !$("canvas-pane").hidden) $("canvas-status").textContent = t("canvasError", error.message);
  }
}

function canvasDocument(result) {
  const stylesheet = String(result.stylesheet || "").replace(/<\/style/gi, "<\\/style");
  const theme = String(result.theme || "").replace(/<\/style/gi, "<\\/style");
  const csp = "default-src 'none'; script-src 'none'; connect-src 'none'; object-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'; navigate-to 'none'; img-src data:; font-src 'none'; style-src 'unsafe-inline'";
  return `<!doctype html><html data-z-ui><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${csp}"><style>${stylesheet}</style><style>${theme}</style></head><body>${result.html}</body></html>`;
}

async function renderCanvas(sequence = ++canvasSequence) {
  if ($("canvas-pane").hidden) return;
  const entry = canvasEntries[Number($("canvas-target").value)];
  if (!entry) return;
  let sample;
  try {
    sample = JSON.parse($("canvas-sample").value);
    if (!sample || Array.isArray(sample) || typeof sample !== "object") throw new Error();
  } catch {
    $("canvas-status").textContent = t("canvasSampleError");
    $("canvas-frame").srcdoc = "";
    return;
  }
  $("canvas-status").textContent = t("canvasLoading");
  $("canvas-source").disabled = true;
  try {
    const result = await api("canvas-render", { method: "POST", body: JSON.stringify({
      ...canvasRequestSource(), kind: entry.kind, name: entry.name, sample,
    }) });
    if (sequence !== canvasSequence || $("canvas-pane").hidden) return;
    $("canvas-frame").srcdoc = canvasDocument(result);
    canvasDeclaration = result.source;
    $("canvas-source").disabled = !result.source?.path;
    $("canvas-status").textContent = t("canvasReady");
  } catch (error) {
    if (sequence !== canvasSequence || $("canvas-pane").hidden) return;
    $("canvas-frame").srcdoc = "";
    canvasDeclaration = null;
    $("canvas-status").textContent = t("canvasError", error.message);
  }
}

function openCanvas() {
  if (matchMedia("(max-width: 760px)").matches && !document.body.classList.contains("sidebar-hidden")) {
    $("sidebar-toggle").click();
  }
  $("canvas-pane").hidden = false;
  $("editor-stage").classList.add("canvas-open");
  requestAnimationFrame(() => editor?.requestMeasure());
  void loadCanvasEntries();
  void loadThemeColors();
}

async function goToCanvasDeclaration() {
  const source = canvasDeclaration;
  if (!source?.path) return;
  await openFile(source.path);
  if (!editor || activePath !== source.path) return;
  const line = Math.max(1, Math.min(editor.state.doc.lines, Number(source.line) || 1));
  editor.dispatch({ selection: { anchor: editor.state.doc.line(line).from }, scrollIntoView: true });
  editor.focus();
}

let capabilityRequest = 0;
let capabilityGrantPreview = null;

function capabilityItem(list, label, statusName = "") {
  const item = document.createElement("li");
  item.textContent = label;
  if (statusName) item.dataset.status = statusName;
  list.append(item);
  return item;
}

async function openCapabilityMatrix() {
  const dialog = $("capability-dialog");
  if (!dialog.open) dialog.showModal();
  const request = ++capabilityRequest;
  const status = $("capability-status");
  status.textContent = t("capabilityLoading");
  $("capability-preview").hidden = true;
  capabilityGrantPreview = null;
  for (const id of ["capability-declared", "capability-needed", "capability-paths", "capability-unused"]) $(id).replaceChildren();
  try {
    const path = activePath?.endsWith(".zyl") ? activePath : studioEntryPath();
    const source = path === activePath ? editor?.state.doc.toString() : undefined;
    const result = await api("capabilities", { method: "POST", body: JSON.stringify(analysisRequest(path, source)) });
    if (request !== capabilityRequest || !dialog.open) return;
    status.textContent = !result.success ? t("capabilityIncomplete") : !result.configured ? t("capabilityUnrestricted") : "";
    const declared = $("capability-declared");
    for (const entry of result.declared || []) {
      capabilityItem(declared, `${entry.function} · ${entry.capability} · ${path}:${entry.line}${entry.granted ? "" : ` · ${t("capabilityNotGranted")}`}`, entry.granted ? "" : "missing");
    }
    if (!declared.childElementCount) capabilityItem(declared, t("capabilityEmpty"));
    const needed = $("capability-needed");
    const missing = new Set();
    for (const entry of result.needed || []) {
      capabilityItem(needed, `${entry.function} · ${entry.operation} · ${entry.capability} · ${path}:${entry.line}${entry.granted ? "" : ` · ${t("capabilityNotGranted")}`}`, entry.granted ? "" : "missing");
      if (!entry.granted) missing.add(entry.capability);
    }
    if (!needed.childElementCount) capabilityItem(needed, t("capabilityEmpty"));
    for (const capability of missing) {
      const button = document.createElement("button");
      button.type = "button"; button.className = "toolbar-button";
      button.textContent = `${t("capabilityGrant")}: ${capability}`;
      button.addEventListener("click", () => void previewCapabilityGrant(capability));
      const item = document.createElement("li"); item.append(button); needed.append(item);
    }
    const paths = $("capability-paths");
    for (const entry of result.filePaths || []) {
      capabilityItem(paths, `${entry.function}:${entry.line} · ${entry.access} · ${entry.path ?? `(${t("capabilityDynamic")})`} · ${entry.status}: ${entry.reason}`, entry.status);
    }
    if (!paths.childElementCount) capabilityItem(paths, t("capabilityEmpty"));
    const unused = $("capability-unused");
    for (const capability of result.grantsWithoutFunctionUse || []) capabilityItem(unused, capability);
    if (unused.childElementCount) capabilityItem(unused, t("capabilityUnusedCaveat"));
    else capabilityItem(unused, t("capabilityEmpty"));
  } catch (error) {
    if (request === capabilityRequest && dialog.open) status.textContent = t("capabilityError", error.message);
  }
}

async function previewCapabilityGrant(capability) {
  const status = $("capability-status");
  try {
    const preview = await api("capability-grant-preview", { method: "POST", body: JSON.stringify({ capability }) });
    if (!$("capability-dialog").open) return;
    capabilityGrantPreview = preview;
    $("capability-preview-label").textContent = `${capability}: ${t("capabilityGrantPreview")}`;
    $("capability-preview-code").textContent = preview.after;
    $("capability-preview").hidden = false;
    $("capability-grant-apply").focus();
  } catch (error) { status.textContent = t("capabilityError", error.message); }
}

async function applyCapabilityGrant() {
  const preview = capabilityGrantPreview;
  if (!preview) return;
  const button = $("capability-grant-apply");
  button.disabled = true;
  try {
    await api("capability-grant-apply", { method: "POST", body: JSON.stringify({
      capability: preview.capability, baseHash: preview.baseHash, after: preview.after, confirm: true,
    }) });
    $("capability-preview").hidden = true;
    capabilityGrantPreview = null;
    await openCapabilityMatrix();
    $("capability-status").textContent = t("capabilityGranted");
  } catch (error) { $("capability-status").textContent = t("capabilityError", error.message); }
  finally { button.disabled = false; }
}

let outboxOffset = 0;
let outboxRequest = 0;
async function loadOutbox() {
  const request = ++outboxRequest;
  const status = $("outbox-status");
  const rows = $("outbox-rows");
  status.textContent = t("outboxLoading");
  rows.replaceChildren();
  $("outbox-previous").disabled = true;
  $("outbox-next").disabled = true;
  try {
    const result = await api(`outbox?offset=${outboxOffset}`);
    if (request !== outboxRequest || !$("outbox-dialog").open) return;
    if (!result.configured) {
      status.textContent = t("outboxNoDatabase");
      $("outbox-page-label").textContent = "";
      return;
    }
    for (const event of result.events || []) {
      const row = document.createElement("tr");
      const cell = (value) => { const td = document.createElement("td"); td.textContent = value || "–"; row.append(td); return td; };
      const eventCell = cell(event.id);
      const kind = document.createElement("small"); kind.textContent = event.type;
      eventCell.append(document.createElement("br"), kind);
      const statusCell = cell(event.status);
      const details = [t("outboxAttempts", event.attempts)];
      if (event.leaseUntil) details.push(t("outboxLeaseUntil", event.leaseUntil));
      if (event.deliveredAt) details.push(t("outboxDeliveredAt", event.deliveredAt));
      if (event.exhaustedAt) details.push(t("outboxExhaustedAt", event.exhaustedAt));
      const detail = document.createElement("small"); detail.textContent = details.join(" · ");
      statusCell.append(document.createElement("br"), detail);
      cell(event.recipient);
      cell(event.attachmentName);
      cell(event.createdAt);
      cell(event.availableAt);
      cell(event.lastError);
      const action = cell("");
      if (event.status === "exhausted") {
        action.textContent = "";
        const button = document.createElement("button");
        button.type = "button"; button.className = "toolbar-button";
        button.textContent = t("outboxRequeue");
        button.addEventListener("click", () => void requeueOutboxEvent(event.id));
        action.append(button);
      }
      rows.append(row);
    }
    status.textContent = result.events?.length ? "" : t("outboxEmpty");
    const shownEnd = outboxOffset + (result.events?.length || 0);
    $("outbox-page-label").textContent = result.total
      ? t("outboxPage", outboxOffset + 1, shownEnd, result.total)
      : t("outboxPage", 0, 0, 0);
    $("outbox-previous").disabled = outboxOffset === 0;
    $("outbox-next").disabled = shownEnd >= result.total || !result.events?.length;
  } catch (error) {
    if (request === outboxRequest && $("outbox-dialog").open) status.textContent = t("outboxLoadError", error.message);
  }
}

function openOutbox() {
  const dialog = $("outbox-dialog");
  if (!dialog.open) dialog.showModal();
  outboxOffset = 0;
  void loadOutbox();
}

async function requeueOutboxEvent(eventId) {
  if (!window.confirm(t("outboxRequeueConfirm", eventId))) return;
  try {
    const result = await api("outbox-requeue", { method: "POST", body: JSON.stringify({
      eventId, confirm: true,
    }) });
    if (!result.requeued || result.sent !== false) throw new Error("Unexpected outbox response");
    if (!$("outbox-dialog").open) return;
    await loadOutbox();
    $("outbox-status").textContent = t("outboxRequeued");
  } catch (error) {
    if ($("outbox-dialog").open) $("outbox-status").textContent = t("outboxRequeueError", error.message);
  }
}

function renderProjectMap(map) {
  const svg = $("project-map-graph");
  const ns = "http://www.w3.org/2000/svg";
  const columns = ["module", "data", "web", "logic"];
  const columnKinds = {
    module: new Set(["module"]),
    data: new Set(["table", "sql", "struct", "class", "form", "tableview", "crud", "view"]),
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
$("semantic-rename-close").addEventListener("click", () => $("semantic-rename-dialog").close());
$("semantic-rename-name").addEventListener("input", () => {
  semanticRenamePlan = null;
  $("semantic-rename-apply-button").disabled = true;
});
$("semantic-rename-name").addEventListener("keydown", (event) => {
  if (event.key === "Enter") { event.preventDefault(); void previewSemanticRename(); }
});
$("semantic-rename-preview-button").addEventListener("click", () => { void previewSemanticRename(); });
$("semantic-rename-apply-button").addEventListener("click", () => { void applySemanticRename(); });
$("delete-file-button").addEventListener("click", deleteActiveFile);
$("save-all-button").addEventListener("click", saveAll);
$("check-button").addEventListener("click", checkProject);
$("run-button").addEventListener("click", runProject);
$("runtime-budget-button").addEventListener("click", () => { void openRuntimeBudget(); });
$("runtime-budget-close").addEventListener("click", () => $("runtime-budget-dialog").close());
$("runtime-budget-run").addEventListener("click", () => {
  $("runtime-budget-dialog").close();
  void runProject();
});
$("runtime-budget-enabled").addEventListener("change", (event) => {
  $("runtime-budget-range").disabled = !event.target.checked;
  $("runtime-budget-number").disabled = !event.target.checked;
});
$("runtime-budget-range").addEventListener("input", (event) => {
  $("runtime-budget-number").value = event.target.value;
});
$("runtime-budget-number").addEventListener("input", (event) => {
  const value = Number(event.target.value);
  if (Number.isFinite(value)) $("runtime-budget-range").value = String(Math.max(1000, Math.min(1_000_000, value)));
});
$("run-output-cancel").addEventListener("click", async () => {
  $("run-output-cancel").disabled = true;
  try {
    const response = await api("cancel-run", { method: "POST", body: "{}" });
    if (response.cancelRequested) $("run-output-state").textContent = t("runCancelRequested");
  } catch (error) {
    announce(error.message, "error");
  }
});
$("run-output-web-stop").addEventListener("click", stopWebPreview);
$("logs-button").addEventListener("click", openEditorLogs);
$("logs-refresh").addEventListener("click", openEditorLogs);
$("logs-level").addEventListener("change", renderEditorLogs);
$("logs-search").addEventListener("input", renderEditorLogs);
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
$("logs-close").addEventListener("click", () => $("logs-dialog").close());
$("project-map-button").addEventListener("click", openProjectMap);
$("project-map-close").addEventListener("click", () => $("project-map-dialog").close());
$("pdf-preview-button").addEventListener("click", openPdfPreview);
$("pdf-preview-close").addEventListener("click", () => $("pdf-preview-dialog").close());
$("pdf-preview-dialog").addEventListener("close", clearPdfPreview);
$("pdf-preview-render").addEventListener("click", renderPdfPreview);
$("outbox-button").addEventListener("click", openOutbox);
$("capability-button").addEventListener("click", () => void openCapabilityMatrix());
$("capability-close").addEventListener("click", () => $("capability-dialog").close());
$("capability-dialog").addEventListener("close", () => { capabilityRequest++; capabilityGrantPreview = null; });
$("capability-grant-apply").addEventListener("click", () => void applyCapabilityGrant());
$("canvas-button").addEventListener("click", openCanvas);
$("canvas-close").addEventListener("click", () => {
  $("canvas-pane").hidden = true;
  $("editor-stage").classList.remove("canvas-open");
  canvasSequence++;
  clearTimeout(canvasTimer);
  requestAnimationFrame(() => editor?.requestMeasure());
});
$("canvas-target").addEventListener("change", () => void renderCanvas());
$("canvas-render").addEventListener("click", () => void renderCanvas());
$("canvas-source").addEventListener("click", () => void goToCanvasDeclaration());
$("canvas-theme-token").addEventListener("change", () => {
  $("canvas-theme-color").value = normalizedColor(canvasColors[Number($("canvas-theme-token").value)]?.color || "#3157a4");
});
$("canvas-theme-apply").addEventListener("click", () => void applyThemeColor());
$("outbox-close").addEventListener("click", () => $("outbox-dialog").close());
$("outbox-dialog").addEventListener("close", () => { outboxRequest++; });
$("outbox-refresh").addEventListener("click", () => void loadOutbox());
$("outbox-previous").addEventListener("click", () => { outboxOffset = Math.max(0, outboxOffset - 25); void loadOutbox(); });
$("outbox-next").addEventListener("click", () => { outboxOffset += 25; void loadOutbox(); });
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
$("database-create").addEventListener("click", createDatabaseFromStudio);
$("database-filter").addEventListener("input", (event) => { databaseFilter = event.target.value; renderDatabaseTables(); });
$("database-record-load").addEventListener("click", () => { void loadDatabaseRecords(0); });
$("database-record-previous").addEventListener("click", () => {
  if (databaseRecordOffsets.length <= 1 || databaseRecordBusy) return;
  databaseRecordOffsets.pop();
  void loadDatabaseRecords(databaseRecordOffsets.at(-1));
});
$("database-record-next").addEventListener("click", () => {
  if (databaseRecordBusy || $("database-record-next").disabled) return;
  void loadDatabaseRecords(databaseRecordOffset + 25);
});
$("chart-preview-button").addEventListener("click", () => { void previewChart(); });
$("chart-save-button").addEventListener("click", () => { void saveChart(); });
$("presentation-add-button").addEventListener("click", addChartToPresentation);
$("presentation-save-button").addEventListener("click", () => { void savePresentation(); });
$("presentation-print-button").addEventListener("click", () => { void printPresentation(); });
$("saved-chart-select").addEventListener("change", () => { $("saved-chart-add").disabled = !$("saved-chart-select").value; });
$("saved-chart-add").addEventListener("click", () => { void loadSavedChart(); });
$("saved-presentation-select").addEventListener("change", () => { $("saved-presentation-load").disabled = !$("saved-presentation-select").value; });
$("saved-presentation-load").addEventListener("click", () => { void loadSavedPresentation(); });
$("chart-aggregation").addEventListener("change", () => { updateChartMeasureAvailability(); invalidateChartPreview(); });
for (const id of ["chart-name", "chart-group", "chart-measure", "chart-kind", "chart-filter-column", "chart-filter-value", "chart-period-column", "chart-period-from", "chart-period-to"]) {
  $(id).addEventListener("input", invalidateChartPreview);
  $(id).addEventListener("change", invalidateChartPreview);
}
$("presentation-name").addEventListener("input", renderPresentationCharts);
$("presentation-note").addEventListener("input", () => { $("presentation-save-button").disabled = !presentationCharts.length; });
$("schema-column-preview").addEventListener("click", previewSchemaColumn);
$("schema-column-apply").addEventListener("click", applySchemaColumn);
$("schema-column-operation").addEventListener("change", updateSchemaColumnOperation);
$("schema-column-existing").addEventListener("change", () => { updateSchemaColumnField(); resetSchemaColumnPreview(); });
for (const id of ["schema-column-name", "schema-column-type", "schema-column-required"]) {
  $(id).addEventListener("input", resetSchemaColumnPreview);
  $(id).addEventListener("change", resetSchemaColumnPreview);
}
$("crud-preview-button").addEventListener("click", previewCrud);
$("crud-apply-button").addEventListener("click", applyCrud);
$("new-file-button").addEventListener("click", createFile);
$("refresh-button").addEventListener("click", refreshTree);
$("tree-filter").addEventListener("input", (event) => { activeFilter = event.target.value; renderTree(); });
$("theme-button").addEventListener("click", () => {
  root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
  localStorage.setItem("zelyra-editor-theme", root.dataset.theme);
  updateThemeControl();
  if (editor) editor.dispatch({ effects: editorTheme.reconfigure(createZelyraTheme()) });
});
$("language-button").addEventListener("click", () => setLanguage(language === "de" ? "en" : "de"));
$("font-size-button").addEventListener("click", cycleFontSize);
$("sidebar-toggle").addEventListener("click", (event) => {
  const hidden = document.body.classList.toggle("sidebar-hidden");
  event.currentTarget.setAttribute("aria-expanded", String(!hidden));
});
$("problems-toggle").addEventListener("click", () => { $("problems-panel").hidden = !$("problems-panel").hidden; });
$("problems-close").addEventListener("click", () => { $("problems-panel").hidden = true; });
$("open-search").addEventListener("click", findInDocument);
window.addEventListener("beforeunload", (event) => {
  eventController?.abort();
  clearTimeout(eventReconnectTimer);
  if (dirtyCount()) { event.preventDefault(); event.returnValue = ""; }
});
window.addEventListener("keydown", (event) => {
  if (event.defaultPrevented) return;
  if ((event.ctrlKey || event.metaKey) && (event.key === "+" || event.key === "=")) {
    event.preventDefault();
    const cur = currentFontSize();
    if (cur === "default") setFontSize("medium");
    else if (cur === "medium") setFontSize("large");
  }
  if ((event.ctrlKey || event.metaKey) && event.key === "-") {
    event.preventDefault();
    const cur = currentFontSize();
    if (cur === "large") setFontSize("medium");
    else if (cur === "medium") setFontSize("default");
  }
  if ((event.ctrlKey || event.metaKey) && event.key === "0") {
    event.preventDefault();
    setFontSize("default");
  }
  if ((event.ctrlKey || event.metaKey) && event.code === "Space" && editor) {
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

setLanguage(language);
$("project-name").textContent = document.body.dataset.project || "Zelyra-Projekt";
$("project-name-side").textContent = (document.body.dataset.project || "ZELYRA-PROJEKT").toLocaleUpperCase();
refreshTree().then(() => {
  const first = treeFiles.includes("main.zyl") ? "main.zyl" : treeFiles.find((file) => file.endsWith(".zyl"));
  if (first) void openFile(first);
});
void connectStudioEvents();
