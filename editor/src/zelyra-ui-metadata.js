const property = (name, type, required = false, defaultValue = "—") => ({
  name, type, required, defaultValue,
});

const appearance = property("appearance", "UiAppearance?", false, "default");
const fieldBase = (type, supportsReadonly = false) => [
  property("id", "String", true),
  property("name", "String", true),
  property("label", "String", true),
  property("value", "String?", false, "leer; bei Passwortfeldern nie vorbefüllt"),
  ...(type === "text" ? [property("placeholder", "String?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), ...(supportsReadonly ? [property("readonly", "Bool?", false, "false")] : [])] : []),
  property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"),
  appearance,
];

export const zelyraUiAppearanceValues = Object.freeze([
  { value: "default", de: "Standard", en: "Default" },
  { value: "success", de: "Erfolg", en: "Success" },
  { value: "warning", de: "Warnung", en: "Warning" },
  { value: "danger", de: "Gefahr", en: "Danger" },
]);

export const zelyraUiComponentMetadata = Object.freeze({
  AppShell: {
    properties: [property("id", "String", true), property("navigation_label", "String", true)],
    slots: ["header", "navigation", "default content", "footer"],
    automatic: "Header, Navigation, Hauptinhalt und Footer bleiben in einem gemeinsamen Layout. Mobile Navigation wird bei verfügbarer JavaScript-Unterstützung eingeklappt.",
  },
  Container: { properties: [], slots: ["default content"], automatic: "Zentrierte Inhaltsbreite aus --z-content-width." },
  PageHeader: { properties: [property("title", "String", true), property("description", "String?")], slots: ["actions"], automatic: "Beschreibung und Aktionen sind optional." },
  Grid: { properties: [property("columns", "UiGridColumns?", false, "one")], slots: ["default content"], automatic: "Eine Spalte; ab 900 px höchstens zwei für drei/vier Spalten; bei 600 px eine Spalte." },
  Stack: { properties: [], slots: ["default content"], automatic: "Vertikaler Abstand aus dem UI-Raster." },
  Inline: { properties: [], slots: ["default content"], automatic: "Zeilenumbruch und Abstand sind responsiv." },
  Toolbar: { properties: [], slots: ["default content"], automatic: "Semantische Toolbar; Aktionen umbrechen bei wenig Platz." },
  Divider: { properties: [], slots: [], automatic: "Dekorative horizontale Trennlinie." },
  Card: { properties: [property("title", "String?")], slots: ["header", "default content", "footer"], automatic: "Kopf- und Fußbereich können leer bleiben." },
  Badge: { properties: [property("text", "String", true), appearance], slots: [], automatic: "Erscheinung default; ändert keine fachliche oder Validierungslogik." },
  Alert: { properties: [property("title", "String", true), appearance], slots: ["default content"], automatic: "Standardmäßig role=status; ErrorState verwendet role=alert." },
  EmptyState: { properties: [property("id", "String", true), property("title", "String", true), property("description", "String?")], slots: ["actions"], automatic: "Die ID verknüpft die Überschrift barrierearm mit dem Zustand." },
  LoadingState: { properties: [property("label", "String", true)], slots: [], automatic: "role=status und aria-live=polite; Animation respektiert reduzierte Bewegung." },
  ErrorState: { properties: [property("title", "String", true), property("message", "String", true)], slots: ["actions"], automatic: "role=alert." },
  Breadcrumbs: { properties: [property("label", "String?")], slots: ["default content"], automatic: "Navigation mit zugänglichem Namen; Inhalte als geordnete Liste." },
  DescriptionList: { properties: [], slots: ["default content"], automatic: "Verwende semantische dt/dd-Paare." },
  Progress: { properties: [property("label", "String", true), property("value", "UInt", true), property("max", "UInt", true)], slots: [], automatic: "Natives progress-Element mit sichtbarer Beschriftung." },
  Button: { properties: [property("label", "String", true), property("href", "String?", false, "nicht gesetzt: echter button type=button"), appearance, property("translation_key", "String?")], slots: [], automatic: "Mit href wird ein Link erzeugt; ohne href ein Button, der kein Formular absendet." },
  SubmitButton: { properties: [property("label", "String", true), appearance, property("translation_key", "String?")], slots: [], automatic: "Explizit type=submit; Erscheinung default." },
  ButtonGroup: { properties: [], slots: ["default content"], automatic: "Gruppiert Aktionen." },
  Link: { properties: [property("href", "String", true), property("label", "String", true)], slots: [], automatic: "Bleibt ein nativer Link." },
  Pagination: { properties: [property("label", "String?")], slots: ["default content"], automatic: "Links und Formulare bleiben ohne JavaScript bedienbar." },
  Tabs: { properties: [property("id", "String", true), property("first_label", "String", true), property("second_label", "String", true)], slots: ["first", "second"], automatic: "Ohne JavaScript sind beide Inhalte sichtbar und Links navigieren zu Fragmenten; mit JavaScript Pfeil/Home/End." },
  Dropdown: { properties: [property("label", "String", true)], slots: ["default content"], automatic: "Natives details/summary; Escape schließt die Liste." },
  Disclosure: { properties: [property("summary", "String", true)], slots: ["default content"], automatic: "Natives details/summary; funktioniert ohne JavaScript." },
  MobileNavigation: { properties: [property("id", "String", true), property("label", "String", true)], slots: ["default content"], automatic: "Navigation bleibt ohne JavaScript sichtbar." },
  TextField: { properties: [...fieldBase("text", true)], slots: ["hint", "error"], automatic: "type=text; required/disabled/readonly sind false, wenn nicht angegeben. invalid bleibt unabhängig von appearance." },
  EmailField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "String?"), property("placeholder", "String?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "type=email; Browserprüfung ergänzt die Servervalidierung." },
  PasswordField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("autocomplete", "String?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "type=password; es gibt keinen value-Prop, damit keine Passwörter erneut ausgegeben werden." },
  NumberField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "String?"), property("min", "String?"), property("max", "String?"), property("step", "String?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "type=number; ohne step gilt die Browservorgabe." },
  DateField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "Date?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "Natives type=date; Darstellung des Kalenders hängt vom Browser/Locale ab." },
  TimeField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "Time?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "Natives type=time." },
  Textarea: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "String?"), property("rows", "UInt?"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("readonly", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "Natives textarea; Höhe passt sich dem Inhalt nicht automatisch an." },
  SelectField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["default options", "hint", "error"], automatic: "Natives select; Optionen müssen als option-Elemente geliefert werden." },
  CheckboxField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "String?"), property("checked", "Bool?", false, "false"), property("required", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: ["hint", "error"], automatic: "Natives checkbox." },
  RadioField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("value", "String", true), property("checked", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: [], automatic: "Natives radio-Eingabefeld; Erscheinungsvariante ändert nur die Akzentfarbe." },
  SwitchField: { properties: [property("id", "String", true), property("name", "String", true), property("label", "String", true), property("checked", "Bool?", false, "false"), property("disabled", "Bool?", false, "false"), property("invalid", "Bool?", false, "nicht gesetzt; Serverfehler separat"), appearance], slots: [], automatic: "Natives checkbox mit role=switch; Erscheinungsvariante ändert nur die Farbe." },
  RadioGroup: { properties: [property("legend", "String", true)], slots: ["default content"], automatic: "Fieldset mit sichtbarer Legend." },
  Fieldset: { properties: [property("legend", "String?")], slots: ["default content"], automatic: "Natives fieldset/legend." },
  FieldGroup: { properties: [property("label", "String?")], slots: ["default content"], automatic: "Gruppiert Formularfelder." },
  FormActions: { properties: [], slots: ["default content"], automatic: "Aktionen umbrechen auf kleinen Ansichten." },
  ErrorSummary: { properties: [property("title", "String", true)], slots: ["default list items"], automatic: "role=alert; Fehler sollten auf Feldanker verweisen." },
  DataTable: { properties: [property("caption", "String", true)], slots: ["head", "body", "foot"], automatic: "Caption, getrennte Tabellenbereiche und ein begrenzt horizontal scrollbarer Bereich." },
  DataTableTools: { properties: [], slots: ["default controls"], automatic: "GET-Formular; Suche und Filter bleiben serverseitig funktionsfähig." },
  Dialog: { properties: [property("id", "String?", false, "deterministische automatische ID"), property("title", "String", true), property("open_label", "String", true), property("close_label", "String?", false, "Schließen (Deutsch), sonst gemäß Dokumentensprache")], slots: ["default content"], automatic: "Native modale Oberfläche mit verknüpftem Titel, Escape, Autofokus-Regeln des Browsers und Fokus-Rückgabe; ohne JavaScript offen verfügbar." },
  ConfirmationForm: { properties: [property("action", "String", true), property("message", "String", true), property("confirm_label", "String", true), property("cancel_href", "String", true)], slots: ["security"], automatic: "POST-Formular; Authentifizierung, CSRF und Berechtigungen müssen serverseitig geprüft werden." },
  ToastRegion: { properties: [], slots: ["default content"], automatic: "Ergänzende Rückmeldung mit aria-live=polite." },
  Toast: { properties: [property("message", "String", true), appearance], slots: [], automatic: "Erscheinung default; keine automatische dauerhafte Statusspeicherung." },
  BarChart: { properties: [property("title", "String", true), property("description", "String", true)], slots: ["ChartBar items"], automatic: "Serverseitig gerendert; jede Balkenanzeige enthält zusätzlich ein natives progress-Element und einen Zahlenwert." },
  ChartBar: { properties: [property("label", "String", true), property("value", "UInt", true), property("max", "UInt", true)], slots: [], automatic: "Natives progress-Element mit sichtbarem Zahlenwert." },
  LineChart: { properties: [property("id", "String", true), property("title", "String", true), property("description", "String", true), property("points", "String", true)], slots: ["data"], automatic: "Inline-SVG mit Titel/Beschreibung; Daten müssen zusätzlich als Tabelle/Text verfügbar sein." },
  DonutChart: { properties: [property("id", "String", true), property("title", "String", true), property("description", "String", true), property("percentage", "UInt", true)], slots: ["data"], automatic: "Inline-SVG mit zugänglicher Beschreibung und Textslot; percentage muss zwischen 0 und 100 liegen." },
  PrintableDocument: { properties: [property("title", "String", true), property("document_number", "String?")], slots: ["default content", "footer"], automatic: "Print-CSS blendet Navigation und Aktionen aus. PDF entsteht über den Browserdialog." },
  PrintAction: { properties: [property("label", "String", true), property("translation_key", "String?")], slots: [], automatic: "Ruft bei verfügbarer JavaScript-Unterstützung window.print() auf; ohne JavaScript bleibt das Dokument druckbar." },
  EmailLink: { properties: [property("recipient", "String", true), property("subject", "String", true), property("body", "String", true), property("label", "String", true), property("translation_key", "String?")], slots: [], automatic: "mailto-Link zum lokalen E-Mail-Programm mit vorausgefülltem Text; PDF-Datei manuell anhängen. Serverseitiger Versand nutzt eine autorisierte Route mit mail_queue und Outbox-Worker." },
});

export function zelyraUiDefaultAppearance(componentName, propertyName) {
  if (propertyName === "appearance") return "default";
  if (componentName === "Grid" && propertyName === "columns") return "one";
  return null;
}
