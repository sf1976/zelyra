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
  const liveChecks = [];
  const databaseRowRequests = [];
  let databaseCreateRequest = null;
  let schemaApplyRequest = null;
  let schemaSourceHash = "";
  let schemaCandidateContent = "";
  const base = `http://127.0.0.1:${port}/__zelyra/editor`;
  const url = `${base}/#${token}`;
  page.on("pageerror", (error) => browserErrors.push(error.message));
  page.on("response", async (response) => {
    if (response.url().endsWith("/api/check")) liveChecks.push({ status: response.status(), body: await response.text() });
  });
  await page.route("**/__zelyra/editor/api/database-schema", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({
      backend: "mariadb",
      modules: [{ source_module: "main.zyl", tables: [{
        name: "customers", status: "matched", declared_columns: [
          { name: "id", type: "BIGINT", nullable: false, primary_key: true, auto_increment: true, unique: false },
          { name: "name", type: "VARCHAR(100)", zelyra_type: "String(100)", zelyra_required: true, nullable: false, primary_key: false, auto_increment: false, unique: false },
        ], live_columns: [
          { name: "id", type: "BIGINT", nullable: false, primary_key: true, auto_increment: true, unique: false },
          { name: "name", type: "VARCHAR(100)", zelyra_type: "String(100)", zelyra_required: true, nullable: false, primary_key: false, auto_increment: false, unique: false },
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
  await page.goto(url);
  assert.equal(new URL(page.url()).hash, "", "the session token must be removed from the address bar");
  await page.locator(".cm-content[contenteditable=true]").waitFor({ timeout: 10_000 });
  await page.locator('.tree-item.file[title="main.zyl"]').waitFor();
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
  await page.locator(".cm-content[contenteditable=true]").waitFor({ timeout: 10_000 });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light", "the selected appearance must survive reload");
  assert.equal(await page.locator(".cm-editor").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)", "CodeMirror must use the selected light appearance");
  await themeButton.focus();
  await page.keyboard.press("Space");
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  assert.equal(await themeButton.getAttribute("aria-label"), "Helles Farbschema aktivieren");
  await page.keyboard.press("Space");
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light", "keyboard users must be able to switch the Studio appearance");
  await page.locator("#task-finder-button").click();
  assert.equal(await page.locator("#task-finder").evaluate((element) => getComputedStyle(element).backgroundColor), "rgb(255, 255, 255)");
  await page.locator("#task-finder-input").fill("no such studio action");
  await page.locator("#task-finder-empty").waitFor();
  await page.locator("#task-finder-input").clear();
  await page.locator("#task-finder-input").fill("format");
  assert.ok(await page.locator(".task-finder-item").count() >= 1, "task finder must show matching actions");
  assert.ok(await page.locator(".task-finder-item").allTextContents().then((items) => items.some((item) => /format/i.test(item))));
  await page.keyboard.press("Escape");
  await page.locator("#task-finder").waitFor({ state: "hidden" });

  const editor = page.locator(".cm-content[contenteditable=true]");
  const initialEditorWidth = await editor.evaluate((element) => element.getBoundingClientRect().width);
  const sidebarToggle = page.locator("#sidebar-toggle");
  await sidebarToggle.click();
  assert.equal(await sidebarToggle.getAttribute("aria-expanded"), "false");
  await page.waitForFunction((width) =>
    document.querySelector(".cm-content")?.getBoundingClientRect().width > width,
  initialEditorWidth);
  await page.setViewportSize({ width: 800, height: 600 });
  await page.waitForFunction(() => document.querySelector(".cm-content")?.getBoundingClientRect().width > 0);
  await page.setViewportSize({ width: 1280, height: 900 });
  await sidebarToggle.click();
  assert.equal(await sidebarToggle.getAttribute("aria-expanded"), "true");

  await editor.focus();
  await page.locator("#complete-button").click();
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
  await page.getByRole("cell", { name: "1" }).waitFor();
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
  await page.route("**/__zelyra/editor/api/run", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ success: true, exitCode: 0, timedOut: false, durationMs: 1, stdout: "3\n", stderr: "", truncated: false }),
  }));
  await page.locator("#run-button").click();
  await page.waitForFunction(() => document.querySelector("#run-output-state")?.dataset.kind === "success");
  const consoleOutput = await page.locator("#run-output-content").textContent();
  assert.match(consoleOutput, /3/, "Studio Run must display console output");
  assert.match(consoleOutput, /interactive input|interaktive Eingabe/i, "Studio must explain that stdin is unavailable");
  await page.unroute("**/__zelyra/editor/api/run");
  await page.locator("#run-output-close").click();

  await editor.focus();
  await page.keyboard.press("Control+A");
  await page.keyboard.type('page "/" { html { <h1>Studio web preview ready</h1> } }\n');
  await page.keyboard.press("Control+S");
  await page.locator(".tab-state.dirty").waitFor({ state: "detached" });
  await page.locator("#run-button").click();
  await page.locator("#run-output-web-link:not([hidden])").waitFor();
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
  await page.locator(".cm-content[contenteditable=true]").waitFor();

  const current = page.locator(".cm-content[contenteditable=true]");
  await current.focus();
  await page.keyboard.press("Control+A");
  await page.keyboard.type("fn main( {\n");
  await page.locator("#problems-panel:not([hidden])").waitFor();
  assert.ok(liveChecks.some((check) => check.status === 200 && JSON.parse(check.body).success === false), "unsaved invalid source must be checked by the compiler");
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
  console.log("Studio database preview, browser editing, unsaved-change warning, keyboard save, diagnostics, and restart passed");
} finally {
  await browser.close();
}
