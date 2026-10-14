import assert from "node:assert/strict";
import { chromium } from "playwright";

const port = process.env.ZELYRA_INVOICE_STUDIO_E2E_PORT;
const token = process.env.ZELYRA_INVOICE_STUDIO_E2E_TOKEN;
assert.ok(port && token, "invoice Studio E2E requires its local session URL");

const base = `http://127.0.0.1:${port}/__zelyra/editor`;
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
  page.setDefaultTimeout(Number(process.env.ZELYRA_INVOICE_STUDIO_E2E_TIMEOUT_MS || 20_000));
  const browserErrors = [];
  page.on("pageerror", (error) => browserErrors.push(error.message));
  await page.goto(`${base}/#${token}`, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').waitFor();

  const runResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/run") && response.request().method() === "POST",
  );
  await page.locator("#run-button").click();
  const response = await runResponse;
  if (response.status() !== 200) {
    throw new Error(`Studio Run returned HTTP ${response.status()}: ${await response.text()}`);
  }
  try {
    await page.locator("#run-output-web-link").waitFor({ state: "visible" });
  } catch (error) {
    throw new Error(`${error.message}\nrun state: ${await page.locator("#run-output-state").textContent()}\nrun output: ${await page.locator("#run-output-content").textContent()}\nbrowser errors: ${JSON.stringify(browserErrors)}`);
  }
  const appUrl = await page.locator("#run-output-web-url").getAttribute("href");
  assert.match(appUrl, /^http:\/\/127\.0\.0\.1:\d+$/);
  await page.locator("#run-output-close").click();

  const targetResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/database-target"),
  );
  await page.locator("#database-button").click();
  const databaseTarget = await (await targetResponse).json();
  assert.equal(databaseTarget.engine, "mariadb");
  assert.equal(databaseTarget.configured, true);
  assert.ok(databaseTarget.database && databaseTarget.database !== "main", "Studio must resolve the actual database name from the generated URL");
  try {
    await page.waitForFunction(() => document.querySelector("#database-state")?.dataset.kind === "success");
  } catch (error) {
    throw new Error(`${error.message}\ndatabase state: ${await page.locator("#database-state").textContent()}\ndatabase tables: ${await page.locator("#database-count").textContent()}\nbrowser errors: ${JSON.stringify(browserErrors)}`);
  }
  assert.match(await page.locator("#database-count").innerText(), /[1-9]\d* Tabellen/);
  assert.ok(await page.locator("#database-tables .database-table-item").count() >= 4, "invoice schema tables must load in Studio");
  await page.locator('#database-tables .database-table-item').filter({ hasText: "customers" }).click();
  await page.locator("#database-record-load").click();
  await page.waitForFunction(() => document.querySelector("#database-record-load")?.disabled === false);
  assert.notEqual(
    await page.locator("#database-record-status").getAttribute("data-kind"),
    "error",
    "customer row preview must succeed even when the table is empty",
  );

  await page.goto(appUrl, { waitUntil: "networkidle" });
  await page.getByRole("heading", { name: /Guten Tag im Rechnungsbüro/ }).waitFor();
  assert.deepEqual(browserErrors, [], "Studio and the invoice application must not emit browser errors");

  const stopResponse = await fetch(`${base}/api/stop-web-preview`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Origin: `http://127.0.0.1:${port}`,
      "X-Zelyra-Editor-Token": token,
    },
    body: "{}",
  });
  assert.equal(stopResponse.status, 200, "preview cleanup must stop the spawned app");
  console.log("invoice Studio database browser, row preview, and first-click application launch passed");
} finally {
  await browser.close();
}
