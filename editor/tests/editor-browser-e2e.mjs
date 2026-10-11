import assert from "node:assert/strict";
import { chromium } from "playwright";

const port = process.env.ZELYRA_STUDIO_E2E_PORT;
const token = process.env.ZELYRA_STUDIO_E2E_TOKEN;
assert.ok(port && token, "Studio browser E2E requires its local session URL");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(5_000);
  const browserErrors = [];
  const liveChecks = [];
  const base = `http://127.0.0.1:${port}/__zelyra/editor`;
  const url = `${base}/#${token}`;
  page.on("pageerror", (error) => browserErrors.push(error.message));
  page.on("response", async (response) => {
    if (response.url().endsWith("/api/check")) liveChecks.push({ status: response.status(), body: await response.text() });
  });
  await page.goto(url);
  assert.equal(new URL(page.url()).hash, "", "the session token must be removed from the address bar");
  await page.locator(".cm-content[contenteditable=true]").waitFor({ timeout: 10_000 });
  await page.locator('.tree-item.file[title="main.zyl"]').waitFor();
  await page.locator("#task-finder-button").click();
  await page.locator("#task-finder-input").fill("no such studio action");
  await page.locator("#task-finder-empty").waitFor();
  await page.locator("#task-finder-input").clear();
  await page.locator("#task-finder-input").fill("format");
  assert.ok(await page.locator(".task-finder-item").count() >= 1, "task finder must show matching actions");
  assert.ok(await page.locator(".task-finder-item").allTextContents().then((items) => items.some((item) => /format/i.test(item))));
  await page.keyboard.press("Escape");
  await page.locator("#task-finder").waitFor({ state: "hidden" });

  const editor = page.locator(".cm-content[contenteditable=true]");
  await editor.focus();
  await page.locator("#complete-button").click();
  await page.locator(".cm-tooltip-autocomplete").waitFor();
  await page.locator(".cm-tooltip-autocomplete .cm-completionLabel", { hasText: /^main$/ }).waitFor();
  await page.keyboard.press("Escape");
  await page.keyboard.press("F1");
  await page.locator("#studio-assistant:not([hidden])").waitFor();
  await page.locator("#assistant-help-details").getByText(/Optionen|Options/).waitFor();
  await page.locator("#assistant-properties-tab").click();
  await page.getByRole("heading", { name: "main.zyl" }).waitFor();
  await page.locator("#assistant-help-tab").click();
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
  console.log("Studio browser editing, unsaved-change warning, keyboard save, diagnostics, and restart passed");
} finally {
  await browser.close();
}
