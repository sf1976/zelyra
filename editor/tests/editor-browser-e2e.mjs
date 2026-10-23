import assert from "node:assert/strict";
import { chromium } from "playwright";

const port = process.env.ZELYRA_STUDIO_E2E_PORT;
const token = process.env.ZELYRA_STUDIO_E2E_TOKEN;
assert.ok(port && token, "Studio browser E2E requires its local session URL");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(10_000);
  const browserErrors = [];
  const databaseRowRequests = [];
  const chartRequests = [];
  let databaseCreateRequest = null;
  const recordMutationRequests = [];
  let schemaApplyRequest = null;
  let schemaSourceHash = "";
  let schemaCandidateContent = "";
  const base = `http://127.0.0.1:${port}/__zelyra/editor`;
  const url = `${base}/#${token}`;
  page.on("pageerror", (error) => browserErrors.push(error.message));
  await page.route("**/__zelyra/editor/api/database-schema", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({
      backend: "mariadb",
      modules: [{ source_module: "main.zyl", tables: [{
        name: "customers", status: "matched", declared_columns: [
          { name: "id", type: "BIGINT", nullable: false, primary_key: true, auto_increment: true, unique: false },
          { name: "name", type: "VARCHAR(100)", zelyra_type: "String(100)", zelyra_required: true, nullable: false, primary_key: false, auto_increment: false, unique: false },
          { name: "amount", type: "DECIMAL(10,2)", sql_type: "DECIMAL(10,2)", zelyra_type: "Decimal", zelyra_required: true, nullable: false, primary_key: false, auto_increment: false, unique: false },
          { name: "created_at", type: "DATE", sql_type: "DATE", zelyra_type: "Date?", nullable: true, primary_key: false, auto_increment: false, unique: false },
        ], live_columns: [
          { name: "id", type: "BIGINT", nullable: false, primary_key: true, auto_increment: true, unique: false },
          { name: "name", type: "VARCHAR(100)", zelyra_type: "String(100)", zelyra_required: true, nullable: false, primary_key: false, auto_increment: false, unique: false },
          { name: "amount", type: "DECIMAL(10,2)", sql_type: "DECIMAL(10,2)", zelyra_type: "Decimal", zelyra_required: true, nullable: false, primary_key: false, auto_increment: false, unique: false },
          { name: "created_at", type: "DATE", sql_type: "DATE", zelyra_type: "Date?", nullable: true, primary_key: false, auto_increment: false, unique: false },
        ], foreign_keys: [], live_foreign_keys: [],
      }] }], unmapped_live_tables: [], crud_resources: [{ table: "customers", name: "CustomerResource" }], crud_targets: ["main.zyl"],
    }),
  }));
  await page.route("**/__zelyra/editor/api/database-target", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ engine: "mariadb", database: "studio_db", configured: true, creationSupported: true }),
  }));
  await page.route("**/__zelyra/editor/api/database-create", async (route) => {
    databaseCreateRequest = route.request().postDataJSON();
    assert.equal(route.request().headers()["x-zelyra-editor-token"], token);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ created: true, database: "studio_db", message: "The declared MariaDB database now exists." }),
    });
  });
  await page.route("**/__zelyra/editor/api/database-rows?**", async (route) => {
    const request = route.request();
    assert.equal(request.headers()["x-zelyra-editor-token"], token, "row browsing must use the local editor session token");
    const url = new URL(request.url());
    databaseRowRequests.push({ table: url.searchParams.get("table"), offset: url.searchParams.get("offset") });
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        readOnly: true,
        columns: ["id", "name"],
        rows: [[url.searchParams.get("offset") === "25" ? "26" : "1", "<img src=x onerror=alert(1)> & Anna"]],
        limit: 25,
        offset: Number(url.searchParams.get("offset")),
        nextOffset: url.searchParams.get("offset") === "25" ? null : 25,
        truncated: false,
      }),
    });
  });
  await page.route("**/__zelyra/editor/api/database-record-mutate", async (route) => {
    const request = route.request();
    assert.equal(request.headers()["x-zelyra-editor-token"], token, "record writes must use the local editor session token");
    const payload = request.postDataJSON();
    recordMutationRequests.push(payload);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ success: true, action: payload.action, message: `${payload.action} complete` }),
    });
  });
  await page.route("**/__zelyra/editor/api/database-chart-preview", async (route) => {
    const request = route.request();
    assert.equal(request.headers()["x-zelyra-editor-token"], token, "chart queries must use the local editor session token");
    const spec = request.postDataJSON();
    chartRequests.push(spec);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ readOnly: true, table: spec.table, groupBy: spec.groupBy, aggregation: spec.aggregation, measure: spec.measure, columns: [spec.groupBy, "value"], rows: [["Anna <script>", "12.5"], ["Bert & Co", "7"]], limit: 20, truncated: false }),
    });
  });
  await page.route("**/__zelyra/editor/api/schema-column-preview", async (route) => {
    const payload = route.request().postDataJSON();
    assert.equal(payload.table, "customers");
    if (payload.operation === "edit") {
      assert.equal(payload.column, "name");
      assert.equal(payload.type, "String(100)");
      assert.equal(payload.required, true);
    } else {
      assert.equal(payload.column, "active");
      assert.equal(payload.operation, "add");
      assert.equal(payload.type, "Bool");
      assert.equal(payload.required, true);
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        success: true,
        candidateHash: "reviewed-schema-preview",
        diff: "+     active: Bool required",
        diagnostics: [],
        impact: { requiresDatabasePlan: true, tables: ["customers"] },
      }),
    });
  });
  await page.route("**/__zelyra/editor/api/schema-column-apply", async (route) => {
    schemaApplyRequest = route.request().postDataJSON();
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        applied: true,
        requiresDatabasePlan: true,
        path: "main.zyl",
        content: schemaCandidateContent,
        hash: schemaSourceHash,
      }),
    });
  });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  assert.equal(new URL(page.url()).hash, "", "the session token must be removed from the address bar");
  const startupSplash = page.locator("#studio-splash");
  await startupSplash.waitFor({ state: "visible" });
  assert.equal(await startupSplash.getAttribute("aria-label"), "Zelyra Studio wird gestartet");
  await page.locator("#empty-state").waitFor({ state: "visible" });
  await page.locator("#empty-state h1", { hasText: "Zelyra verstehen" }).waitFor();
  await page.locator('.tree-item.file[title="main.zyl"]').waitFor();
  await page.waitForFunction(() => document.querySelector("#welcome-project-status")?.textContent.includes("vollständig analysiert"));
  assert.equal(await page.locator("#welcome-project-modules").textContent(), "2", "the welcome view counts compiler-loaded project modules");
  assert.equal(await page.locator("#welcome-project-data").textContent(), "3", "the welcome view counts declared tables and records");
  assert.equal(await page.locator("#welcome-project-web").textContent(), "3", "the welcome view counts declared web surfaces and reusable views");
  assert.equal(await page.locator("#welcome-project-sql").textContent(), "1", "the welcome view counts parsed SQL declarations");
  assert.equal(await page.locator("#welcome-project-action").getAttribute("data-action"), "map", "the compiler model suggests the project map when web queries exist");
  await page.locator("#welcome-project-action").click();
  await page.locator("#project-map-dialog").waitFor({ state: "visible" });
  await page.waitForFunction(() => document.querySelector("#project-map-status")?.textContent === "");
  await page.locator("#project-map-close").click();
  await page.route("**/__zelyra/editor/api/project-map", async (route) => {
    const response = await route.fetch();
    const model = await response.json();
    model.incomplete = true;
    await route.fulfill({ response, body: JSON.stringify(model) });
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => document.querySelector("#welcome-project-status")?.textContent.includes("Analyse unvollständig"));
  assert.equal(await page.locator("#welcome-project-action").getAttribute("data-action"), "check", "incomplete compiler analysis recommends a project check");
  assert.equal(await page.locator("#welcome-project-action").textContent(), "Projekt prüfen");
  assert.match(await page.locator("#welcome-project-guidance").textContent(), /nicht vollständig auflösbar/);
  await page.unroute("**/__zelyra/editor/api/project-map");
  await startupSplash.waitFor({ state: "detached", timeout: 5_000 });
  await page.locator("#welcome-terminal").click();
  await page.locator("#terminal-panel").waitFor({ state: "visible" });
  await page.locator("#terminal-start").click();
  await page.locator("#terminal-input").waitFor({ state: "visible" });
  if (process.env.ZELYRA_STUDIO_E2E_BACKEND === "postgres") {
    await page.waitForFunction(() => document.querySelector("#terminal-output")?.textContent.includes("sandbox transactions currently require MariaDB/MySQL"));
  } else {
    await page.locator("#terminal-input").fill("1 + 1");
    await page.locator("#terminal-input").press("Enter");
    await page.waitForFunction(() => document.querySelector("#terminal-output")?.textContent.includes("=> 2"));
    await page.locator("#terminal-input").fill(":quit");
    await page.locator("#terminal-input").press("Enter");
    await page.waitForFunction(() => document.querySelector("#terminal-stop")?.disabled === true);
  }
  await page.locator("#terminal-close").click();
  await page.locator("#welcome-open-file").click();
  await page.locator(".cm-content[contenteditable=true]").waitFor({ timeout: 10_000 });
  const themeButton = page.locator("#theme-button");
  await themeButton.click();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
  assert.equal(await themeButton.getAttribute("aria-label"), "Dunkles Farbschema aktivieren");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  assert.equal(await page.locator(".topbar").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  assert.equal(await page.locator(".explorer").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(248, 249, 252)");
  assert.equal(await page.locator("#project-map-dialog").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  const lightThemeContrast = await page.locator("html").evaluate((root) => {
    const css = getComputedStyle(root);
    const rgb = (name) => {
      const hex = css.getPropertyValue(name).trim().match(/^#([\da-f]{3}|[\da-f]{6})$/i)?.[1];
      if (!hex) throw new Error(`light theme color ${name} is not a hex color`);
      const channels = hex.length === 3 ? [...hex].map((part) => part + part) : hex.match(/[\da-f]{2}/gi);
      return channels.map((part) => Number.parseInt(part, 16) / 255);
    };
    const luminance = (color) => color.map((channel) => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4).reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);
    const foregrounds = ["--ink", "--muted", "--faint", "--accent", "--danger", "--success", "--warning", "--token-keyword", "--token-type", "--token-name", "--token-string", "--token-number", "--token-comment", "--token-operator", "--token-punctuation"];
    const backgrounds = ["--canvas", "--surface", "--surface-raised", "--editor-bg", "--editor-gutter", "--sidebar"];
    return Object.fromEntries(foregrounds.map((foreground) => [foreground, Object.fromEntries(backgrounds.map((background) => {
      const contrast = (Math.max(luminance(rgb(foreground)), luminance(rgb(background))) + 0.05) / (Math.min(luminance(rgb(foreground)), luminance(rgb(background))) + 0.05);
      return [background, contrast];
    }))]));
  });
  assert.ok(Object.values(lightThemeContrast).flatMap(Object.values).every((ratio) => ratio >= 4.5), `light-theme text contrast must be at least 4.5:1: ${JSON.stringify(lightThemeContrast)}`);
  await page.reload();
  await page.locator("#empty-state").waitFor({ state: "visible" });
  await page.locator("#welcome-open-file").click();
  await page.locator(".cm-content[contenteditable=true]").waitFor({ timeout: 10_000 });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light", "the selected appearance must survive reload");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)", "CodeMirror must use the selected light appearance");
  await themeButton.focus();
  await page.keyboard.press("Space");
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  assert.equal(await themeButton.getAttribute("aria-label"), "Helles Farbschema aktivieren");
  await page.locator(".cm-content[contenteditable=true]").focus();
  await page.keyboard.press("F1");
  await page.locator("#assistant-help-view").waitFor({ state: "visible" });
  assert.equal(await page.locator("#studio-assistant").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(17, 24, 42)");
  await page.keyboard.press("F1");
  await page.locator("#assistant-help-view").waitFor({ state: "hidden" });
  await themeButton.focus();
  await page.keyboard.press("Space");
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light", "keyboard users must be able to switch the Studio appearance");

  const fontSizeButton = page.locator("#font-size-button");
  const changeFontSize = async () => {
    await page.locator('[data-studio-menu="view"]').click();
    await page.locator('[data-menu-target="font-size-button"]').click();
  };
  assert.equal(await fontSizeButton.textContent(), "aA");
  assert.equal(await page.locator("html").getAttribute("data-font-size"), null);
  await changeFontSize();
  assert.equal(await page.locator("html").getAttribute("data-font-size"), "medium");
  assert.equal(await fontSizeButton.textContent(), "aA⁺");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).fontSize), "16px");

  await changeFontSize();
  assert.equal(await page.locator("html").getAttribute("data-font-size"), "large");
  assert.equal(await fontSizeButton.textContent(), "aA⁺⁺");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).fontSize), "18px");

  await changeFontSize();
  assert.equal(await page.locator("html").getAttribute("data-font-size"), null);
  assert.equal(await fontSizeButton.textContent(), "aA");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).fontSize), "14px");

  await page.keyboard.press("Control+=");
  assert.equal(await page.locator("html").getAttribute("data-font-size"), "medium");
  await page.keyboard.press("Control+=");
  assert.equal(await page.locator("html").getAttribute("data-font-size"), "large");

  await page.reload();
  await page.locator("#empty-state").waitFor({ state: "visible" });
  await page.locator("#welcome-open-file").click();
  await page.locator(".cm-content[contenteditable=true]").waitFor({ timeout: 10_000 });
  assert.equal(await page.locator("html").getAttribute("data-font-size"), "large", "the selected font size must survive reload");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).fontSize), "18px");

  await page.keyboard.press("Control+-");
  assert.equal(await page.locator("html").getAttribute("data-font-size"), "medium");
  await page.keyboard.press("Control+0");
  assert.equal(await page.locator("html").getAttribute("data-font-size"), null);
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).fontSize), "14px");

  const openTaskFinder = async () => {
    await page.locator('[data-studio-menu="help"]').click();
    await page.locator('[data-menu-target="task-finder-button"]').click();
  };
  await openTaskFinder();
  assert.equal(await page.locator("#task-finder").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  await page.locator("#task-finder-input").fill("no such studio action");
  await page.locator("#task-finder-empty").waitFor();
  await page.locator("#task-finder-input").clear();
  await page.locator("#task-finder-input").fill("format");
  assert.ok(await page.locator(".task-finder-item").count() >= 1, "task finder must show matching actions");
  assert.ok(await page.locator(".task-finder-item").allTextContents().then((items) => items.some((item) => /format/i.test(item))));
  await page.keyboard.press("Escape");
  await page.locator("#task-finder").waitFor({ state: "hidden" });
  await openTaskFinder();
  await page.locator("#task-finder-input").fill("properties");
  await page.locator(".task-finder-item", { hasText: /Dateieigenschaften öffnen/ }).click();
  await page.locator("#assistant-properties-tab").waitFor({ state: "visible" });
  assert.equal(await page.locator("#studio-assistant").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  await openTaskFinder();
  await page.locator("#task-finder-input").fill("properties");
  await page.locator(".task-finder-item", { hasText: /Dateieigenschaften öffnen/ }).click();
  await page.locator("#assistant-properties-tab").waitFor({ state: "hidden" });

  const editor = page.locator(".cm-content[contenteditable=true]");
  const initialEditorWidth = await editor.evaluate((element) => element.getBoundingClientRect().width);
  const sidebarToggle = page.locator("#sidebar-toggle");
  await sidebarToggle.click();
  assert.equal(await sidebarToggle.getAttribute("aria-expanded"), "false");
  await page.waitForFunction((width) =>
    document.querySelector(".cm-content")?.getBoundingClientRect().width > width,
  initialEditorWidth);
  for (const width of [360, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    await page.waitForFunction((expected) => window.innerWidth === expected, width);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), `Studio must not overflow horizontally at ${width}px`);
    if (width === 360) assert.ok(await page.locator("#theme-button").isVisible(), "theme control must remain reachable on a narrow screen");
  }
  await sidebarToggle.click();
  assert.equal(await sidebarToggle.getAttribute("aria-expanded"), "true");

  await editor.focus();
  await page.locator('[data-studio-menu="edit"]').click();
  await page.locator('[data-menu-target="complete-button"]').click();
  await page.locator(".cm-tooltip-autocomplete").waitFor();
  const mainCompletions = page.locator(".cm-tooltip-autocomplete .cm-completionLabel", { hasText: /^main$/ });
  await mainCompletions.first().waitFor();
  assert.equal(await mainCompletions.count(), 1, "completion must deduplicate the project entry point");
  await page.keyboard.press("Escape");

  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.waitForFunction(() =>
    document.querySelector("#active-file")?.textContent === "main.zyl" &&
    document.querySelector(".cm-content")?.textContent.includes("import"),
  );
  await page.locator(".cm-scroller").evaluate((element) => { element.scrollTop = element.scrollHeight; });
  const callSite = page.locator(".cm-line", { hasText: "math::sum" });
  await callSite.waitFor();
  await callSite.click();
  const callSiteOffset = (await callSite.textContent()).trimStart().indexOf("sum");
  await page.keyboard.press("Home");
  for (let index = 0; index < callSiteOffset; index += 1) await page.keyboard.press("ArrowRight");
  for (let index = 0; index < 3; index += 1) await page.keyboard.press("Shift+ArrowRight");
  await page.keyboard.press("F12");
  await page.waitForFunction(() => document.querySelector("#active-file")?.textContent === "src/math.zyl");
  await page.keyboard.press("Shift+F12");
  await page.locator("#project-search").waitFor({ state: "visible" });
  const references = page.locator("#project-search-results .task-finder-item");
  await page.waitForFunction(() => document.querySelectorAll("#project-search-results .task-finder-item").length >= 2);
  const referenceRows = await references.allTextContents();
  assert.ok(referenceRows.some((row) => row.includes("src/math.zyl")), "references must include the declaration file");
  assert.ok(referenceRows.some((row) => row.includes("main.zyl")), "references must include the imported call site");
  await page.locator("#project-search-close").click();
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.waitForFunction(() => document.querySelector("#active-file")?.textContent === "main.zyl");

  await page.locator("#database-button").click();
  await page.locator("#database-create").waitFor({ state: "visible" });
  assert.equal(await page.locator(".database-header").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  page.once("dialog", async (dialog) => {
    assert.match(dialog.message(), /studio_db/);
    await dialog.accept();
  });
  await page.locator("#database-create").click();
  await page.waitForFunction(() => document.querySelector("#database-target-state")?.textContent.includes("Datenbank vorhanden"));
  assert.equal(databaseCreateRequest.confirm, true, "database creation must require explicit confirmation");
  const customerTable = page.locator(".database-table-item", { hasText: "customers" });
  await customerTable.waitFor();
  await customerTable.click();
  await page.locator("#database-record-browser").waitFor({ state: "visible" });
  assert.deepEqual(databaseRowRequests, [], "Studio must not fetch table data before the user asks");
  await page.locator("#database-record-load").click();
  await page.getByRole("cell", { name: "<img src=x onerror=alert(1)> & Anna" }).waitFor();
  assert.equal(await page.locator("#database-record-table-wrap img").count(), 0, "database values must render as text, not markup");
  await page.locator("#database-record-next").click();
  await page.getByRole("cell", { name: "26" }).waitFor();
  assert.deepEqual(databaseRowRequests, [
    { table: "customers", offset: "0" },
    { table: "customers", offset: "25" },
  ]);
  await page.locator("#database-record-previous").click();
  await page.getByRole("cell", { name: "1", exact: true }).waitFor();
  await page.locator("#database-record-create").click();
  await page.locator("#database-record-dialog").waitFor({ state: "visible" });
  await page.locator('#database-record-fields [data-column="name"]').fill("New customer");
  await page.locator('#database-record-fields [data-column="amount"]').fill("19.95");
  page.once("dialog", async (dialog) => { assert.match(dialog.message(), /Datensatz.*anlegen/); await dialog.accept(); });
  const createMutation = page.waitForRequest((request) => request.url().includes("/api/database-record-mutate") && request.postDataJSON().action === "create");
  await page.locator("#database-record-save").click();
  await createMutation;
  await page.locator("#database-record-dialog").waitFor({ state: "hidden" });
  const editButtons = page.locator("#database-record-table-wrap button", { hasText: "Bearbeiten" });
  await editButtons.first().click();
  await page.locator('#database-record-fields [data-column="name"]').fill("Edited customer");
  page.once("dialog", async (dialog) => { assert.match(dialog.message(), /Änderungen.*speichern/); await dialog.accept(); });
  const updateMutation = page.waitForRequest((request) => request.url().includes("/api/database-record-mutate") && request.postDataJSON().action === "update");
  await page.locator("#database-record-save").click();
  await updateMutation;
  await page.locator("#database-record-dialog").waitFor({ state: "hidden" });
  page.once("dialog", async (dialog) => { assert.match(dialog.message(), /dauerhaft löschen/); await dialog.accept(); });
  const deleteMutation = page.waitForRequest((request) => request.url().includes("/api/database-record-mutate") && request.postDataJSON().action === "delete");
  await page.locator("#database-record-table-wrap button", { hasText: "Löschen" }).first().click();
  await deleteMutation;
  assert.deepEqual(recordMutationRequests.map(({ action, confirm }) => [action, confirm]), [
    ["create", true], ["update", true], ["delete", true],
  ]);
  await page.locator("#schema-column-operation").selectOption("edit");

  await page.locator("#chart-name").fill("Umsatz nach Kunde");
  await page.locator("#chart-group").selectOption("name");
  await page.locator("#chart-aggregation").selectOption("sum");
  await page.locator("#chart-measure").selectOption("amount");
  await page.locator("#chart-kind").selectOption("line");
  await page.locator("#chart-filter-column").selectOption("name");
  await page.locator("#chart-filter-value").fill("Anna' OR 1=1 --");
  await page.locator("#chart-period-column").selectOption("created_at");
  await page.locator("#chart-period-from").fill("2026-10-01");
  await page.locator("#chart-period-to").fill("2026-10-31");
  await page.locator("#chart-preview-button").click();
  try {
    await page.locator("#chart-svg .chart-line").waitFor();
  } catch (error) {
    throw new Error(`${error.message}\nchart state: ${await page.locator("#chart-state").textContent()}\nrequests: ${JSON.stringify(chartRequests)}\nbrowser errors: ${JSON.stringify(browserErrors)}`);
  }
  assert.equal(await page.locator("#chart-data-table-wrap").innerText(), "Gruppe\tWert\nAnna <script>\t12,5\nBert & Co\t7");
  assert.equal(await page.locator("#chart-data-table-wrap script").count(), 0, "chart labels must render as text, not markup");
  assert.equal(chartRequests.at(-1).aggregation, "sum");
  assert.equal(chartRequests.at(-1).measure, "amount");
  assert.deepEqual(chartRequests.at(-1).filter, { column: "name", value: "Anna' OR 1=1 --" });
  assert.deepEqual(chartRequests.at(-1).dateRange, { column: "created_at", from: "2026-10-01", to: "2026-10-31" });
  await page.locator("#chart-save-button").click();
  try {
    await page.waitForFunction(() => document.querySelector("#chart-state")?.textContent.includes("chart-umsatz-nach-kunde.zchart.json"));
  } catch (error) {
    throw new Error(`${error.message}\nchart state after save: ${await page.locator("#chart-state").textContent()}\nstatus bar: ${await page.locator("#status-message").textContent()}\nbrowser errors: ${JSON.stringify(browserErrors)}`);
  }
  await page.locator("#saved-chart-select").selectOption("chart-umsatz-nach-kunde.zchart.json");
  await page.locator("#saved-chart-add").click();
  await page.waitForFunction(() => document.querySelectorAll("#presentation-slide-list .presentation-slide-chip").length === 1);
  assert.equal(chartRequests.at(-1).title, "Umsatz nach Kunde", "loading a saved chart must restore and recheck its configuration");

  await page.locator("#chart-name").fill("Kundenzahl nach Name");
  await page.locator("#chart-aggregation").selectOption("count");
  await page.locator("#chart-kind").selectOption("bar");
  await page.locator("#chart-filter-column").selectOption("");
  await page.locator("#chart-filter-value").clear();
  await page.locator("#chart-period-column").selectOption("");
  await page.locator("#chart-period-from").fill("");
  await page.locator("#chart-period-to").fill("");
  await page.locator("#chart-preview-button").click();
  await page.locator("#chart-svg .chart-bar").first().waitFor();
  await page.locator("#presentation-add-button").click();
  await page.locator("#presentation-name").fill("Monatsbericht");
  await page.locator("#presentation-note").fill("Die wichtigsten Kennzahlen im Überblick.");
  await page.locator("#presentation-save-button").click();
  await page.getByText("Präsentation gespeichert: presentation-monatsbericht.zelyra-presentation.json").waitFor();
  await page.locator("#presentation-note").fill("Die wichtigsten Kennzahlen im Überblick, aktualisiert.");
  const presentationUpdate = page.waitForResponse((response) =>
    response.url().includes("/api/file") && response.request().method() === "PUT",
  );
  await page.locator("#presentation-save-button").click();
  assert.equal((await presentationUpdate).status(), 200, "updating a saved Zelyra presentation must use the hash-checked file API");
  const savedPresentation = await page.evaluate(async ({ base, token }) => {
    const response = await fetch(`${base}/api/file?path=presentation-monatsbericht.zelyra-presentation.json`, { headers: { "X-Zelyra-Editor-Token": token } });
    return response.json();
  }, { base, token });
  const presentation = JSON.parse(savedPresentation.content);
  assert.equal(presentation.schema, "zelyra.presentation.v1");
  assert.equal(presentation.charts.length, 2, "presentations must preserve several reviewed chart configurations");
  assert.equal(presentation.introduction, "Die wichtigsten Kennzahlen im Überblick, aktualisiert.", "saving a report again must update the existing Zelyra report safely");
  await page.locator("#presentation-name").fill("Geänderter Titel");
  await page.locator("#saved-presentation-select").selectOption("presentation-monatsbericht.zelyra-presentation.json");
  await page.locator("#saved-presentation-load").click();
  await page.waitForFunction(() => document.querySelector("#presentation-name")?.value === "Monatsbericht");
  assert.equal(await page.locator("#presentation-note").inputValue(), "Die wichtigsten Kennzahlen im Überblick, aktualisiert.");
  assert.equal(await page.locator("#presentation-slide-list .presentation-slide-chip").count(), 2);
  const printPopupPromise = page.waitForEvent("popup");
  await page.locator("#presentation-print-button").click();
  const printPopup = await printPopupPromise;
  await printPopup.locator(".chart-print-page").nth(2).waitFor();
  assert.equal(await printPopup.locator("h1").innerText(), "Monatsbericht");
  assert.equal(await printPopup.locator(".chart-print-page").count(), 3, "the print view includes a cover and both chart slides");
  const pdf = await printPopup.pdf({ format: "A4", printBackground: true });
  assert.ok(pdf.subarray(0, 4).toString() === "%PDF", "the presentation print view must produce a PDF");
  await printPopup.close();

  await page.locator("#schema-column-operation").selectOption("edit");
  await page.locator("#schema-column-existing").selectOption("name");
  assert.equal(await page.locator("#schema-column-name").inputValue(), "name");
  assert.equal(await page.locator("#schema-column-name").isEditable(), false);
  await page.locator("#schema-column-preview").click();
  await page.waitForFunction(() => document.querySelector("#schema-column-apply") && !document.querySelector("#schema-column-apply").disabled);
  await page.locator("#schema-column-operation").selectOption("add");
  const currentFile = await page.evaluate(async ({ base, token }) => {
    const response = await fetch(`${base}/api/file?path=main.zyl`, { headers: { "X-Zelyra-Editor-Token": token } });
    return response.json();
  }, { base, token });
  schemaSourceHash = currentFile.hash;
  schemaCandidateContent = `${currentFile.content}\ntable customers {\n    id: Id primary auto\n    active: Bool required\n}\n`;
  await page.locator("#schema-column-name").fill("active");
  await page.locator("#schema-column-type").selectOption("Bool");
  await page.locator("#schema-column-required").check();
  await page.locator("#schema-column-preview").click();
  await page.waitForFunction(() => document.querySelector("#schema-column-apply") && !document.querySelector("#schema-column-apply").disabled);
  page.once("dialog", async (dialog) => {
    assert.match(dialog.message(), /active/);
    await dialog.accept();
  });
  await page.locator("#schema-column-apply").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent.includes("Quelltext geändert"));
  assert.equal(schemaApplyRequest.confirm, true, "source changes must carry explicit confirmation");
  assert.equal(schemaApplyRequest.candidateHash, "reviewed-schema-preview");
  await page.locator("#language-button").click();
  await page.getByRole("heading", { name: "Browse records" }).waitFor();
  await page.locator("#editor-stage").waitFor({ state: "hidden" });
  await page.locator("#code-button").click();
  await page.locator("#editor-stage").waitFor({ state: "visible" });

  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.waitForFunction(() => document.querySelector("#active-file")?.textContent === "main.zyl");

  await page.locator(".cm-scroller").evaluate((element) => { element.scrollTop = element.scrollHeight; });
  const functionDeclaration = page.locator(".cm-line", { hasText: "fn main" });
  await functionDeclaration.waitFor();
  await functionDeclaration.click();
  await page.keyboard.press("Home");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("F1");
  await page.locator("#studio-assistant:not([hidden])").waitFor();
  await page.locator("#assistant-help-details").getByText(/Optionen|Options/).waitFor();
  await page.locator("#assistant-properties-tab").click();
  await page.getByRole("heading", { name: "main.zyl" }).waitFor();
  await page.locator("#assistant-help-tab").click();
  const codeEditor = page.locator(".cm-content[contenteditable=true]");
  await codeEditor.fill('import "zelyra/ui" as ui\npage "/" {\n    html {\n        <Button label="Continue" />\n    }\n}\n');
  const componentLine = page.locator(".cm-line", { hasText: "<Button label" });
  await componentLine.waitFor();
  await componentLine.click();
  await page.keyboard.press("Home");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await page.locator("#assistant-properties-tab").click();
  await page.getByRole("heading", { name: "<Button>" }).waitFor();
  await page.locator("#assistant-ui-appearance").selectOption("danger");
  await page.locator("#assistant-ui-appearance-apply").click();
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent.includes('appearance="danger"'));
  await page.getByText(/ohne href|without href/i).waitFor();
  await page.route("**/__zelyra/editor/api/run", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ success: true, exitCode: 0, timedOut: false, durationMs: 1, stdout: "3\n", stderr: "", truncated: false }),
  }));
  await page.locator("#run-button").click();
  try {
    await page.waitForFunction(() => document.querySelector("#run-output-state")?.dataset.kind === "success");
  } catch (error) {
    throw new Error(`${error.message}\nrun state: ${await page.locator("#run-output-state").textContent()}\nstatus: ${await page.locator("#status-message").textContent()}\nbrowser errors: ${JSON.stringify(browserErrors)}`);
  }
  assert.equal(await page.locator("#run-output-state").evaluate((element) => getComputedStyle(element).fontSize), "12px");
  assert.equal(await page.locator("#run-output-content").evaluate((element) => getComputedStyle(element).fontSize), "13px", "program output text should be comfortably readable without changing editor source text");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).fontSize), "14px", "program output sizing must not alter the code editor font");
  const consoleOutput = await page.locator("#run-output-content").textContent();
  assert.match(consoleOutput, /3/, "Studio Run must display console output");
  assert.equal(await page.locator("#run-console-form").isVisible(), false, "completed runs must hide interactive input controls");
  await page.unroute("**/__zelyra/editor/api/run");
  await page.locator("#run-output-close").click();

  await editor.focus();
  await page.keyboard.press("Control+A");
  await page.keyboard.type('page "/" { html { <h1>Studio web preview ready</h1> } }\n');
  await page.keyboard.press("Control+S");
  await page.locator(".tab-state.dirty").waitFor({ state: "detached" });
  await page.locator("#run-button").click();
  try {
    await page.locator("#run-output-web-link:not([hidden])").waitFor();
  } catch (error) {
    throw new Error(`${error.message}\nrun state: ${await page.locator("#run-output-state").textContent()}\nrun output: ${await page.locator("#run-output-content").textContent()}\nbrowser errors: ${JSON.stringify(browserErrors)}`);
  }
  const previewUrl = await page.locator("#run-output-web-url").getAttribute("href");
  assert.match(previewUrl, /^http:\/\/127\.0\.0\.1:\d+$/);
  const previewResponse = await page.request.get(previewUrl);
  assert.equal(previewResponse.status(), 200);
  assert.match(await previewResponse.text(), /Studio web preview ready/);
  await page.locator("#run-output-web-stop").click();
  await page.locator("#run-output-web-link").waitFor({ state: "hidden" });
  await assert.rejects(() => page.request.get(previewUrl, { timeout: 1_000 }));
  await page.locator("#run-output-close").click();

  await editor.focus();
  await page.keyboard.press("Control+A");
  await page.keyboard.type('fn main() { print("unsaved browser edit"); }\n');
  await page.locator(".tab-state.dirty").waitFor();

  let unloadPromptSeen = false;
  page.once("dialog", async (dialog) => {
    unloadPromptSeen = dialog.type() === "beforeunload";
    await dialog.accept();
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  assert.equal(unloadPromptSeen, true, "reload must warn before discarding unsaved edits");
  await page.locator("#empty-state").waitFor({ state: "visible" });
  await page.locator("#welcome-open-file").click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();

  const current = page.locator(".cm-content[contenteditable=true]");
  await current.focus();
  await page.keyboard.press("Control+A");
  const invalidCompilerResponse = page.waitForResponse(async (response) => {
    if (!response.url().endsWith("/api/check") || response.status() !== 200) return false;
    try {
      return (await response.json()).success === false;
    } catch {
      return false;
    }
  });
  await page.keyboard.type("fn main( {\n");
  await invalidCompilerResponse;
  await page.locator("#problems-panel:not([hidden])").waitFor();
  assert.ok(await page.locator(".tab-state.dirty").count(), "live checking must leave unsaved source untouched");
  await current.press("Control+S");
  await page.locator(".tab-state.dirty").waitFor({ state: "detached" });
  await page.getByRole("button", { name: /Projekt prüfen|Check project/ }).click();
  await page.waitForFunction(() => !document.querySelector("#status-message").textContent.includes("wird geprüft") && !document.querySelector("#status-message").textContent.includes("Checking project"));
  console.log("Studio check status:", await page.locator("#status-message").textContent());
  await page.locator("#problems-panel:not([hidden])").waitFor();
  await page.locator("#problems-count").waitFor();
  const problemCount = Number(await page.locator("#problems-count").textContent());
  assert.ok(problemCount > 0, "malformed source must produce visible compiler diagnostics");

  await page.reload({ waitUntil: "domcontentloaded" });
  await page.locator("#empty-state").waitFor({ state: "visible" });
  await page.locator("#welcome-open-file").click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  assert.equal(
    await page.locator(".cm-content[contenteditable=true]").textContent(),
    "fn main( {",
    "saved content must be restored after a clean Studio restart",
  );

  const response = await page.evaluate(async ({ base, token }) => {
    const result = await fetch(`${base}/api/file?path=main.zyl`, {
      headers: { "X-Zelyra-Editor-Token": token },
    });
    return result.json();
  }, { base, token });
  assert.equal(response.content, "fn main( {\n", "the browser keyboard save must reach the project file");
  assert.deepEqual(browserErrors, [], "Studio must not raise browser runtime errors");
  console.log("Studio welcome, Zelyra terminal, database CRUD, browser editing, unsaved-change warning, keyboard save, diagnostics, and restart passed");
} finally {
  await browser.close();
}
