import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { chromium } from "playwright";

const baseUrl = process.env.ZELYRA_TENANT_BROWSER_URL;
const email = process.env.ZELYRA_TENANT_BROWSER_EMAIL;
const password = process.env.ZELYRA_TENANT_BROWSER_PASSWORD;
const documentId = process.env.ZELYRA_TENANT_BROWSER_DOCUMENT_ID;
const expectedFile = process.env.ZELYRA_TENANT_BROWSER_FILE;
for (const [name, value] of Object.entries({ baseUrl, email, password, documentId, expectedFile })) {
  assert.ok(value, `${name} must be configured by the tenant CRUD E2E`);
}

const browser = await chromium.launch({ headless: true });
try {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  page.setDefaultTimeout(8_000);
  page.setDefaultNavigationTimeout(8_000);
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(`${baseUrl}/login`, { waitUntil: "domcontentloaded" });
  const loginCsrf = await page.locator('input[name="_zelyra_csrf"]').inputValue();
  assert.ok(loginCsrf, "the login form contains its CSRF token");
  await page.locator('input[name="email"]').fill(email);
  await page.locator('input[name="password"]').fill(password);
  await Promise.all([
    page.waitForResponse((response) => response.url().endsWith("/login") && response.request().method() === "POST"),
    page.locator('button[type="submit"]').click(),
  ]);

  await page.goto(`${baseUrl}/tenant_documents/new?tenant_id=20`, { waitUntil: "domcontentloaded" });
  assert.equal(await page.locator('input[type="file"][name="attachment"]').count(), 1,
    "the no-JavaScript create page renders its generated attachment input");

  const detailUrl = `${baseUrl}/tenant_documents/${documentId}?tenant_id=20`;
  await page.goto(detailUrl, { waitUntil: "domcontentloaded" });
  const fileUrl = `/tenant_documents/${documentId}/files/attachment?tenant_id=20`;
  const link = page.locator(`a[href="${fileUrl}"]`);
  assert.equal(await link.count(), 1, "the authorized detail page renders the tenant-scoped download link");
  const browserDownloadPromise = page.waitForEvent("download");
  const downloadResponsePromise = page.waitForResponse((response) =>
    response.url() === `${baseUrl}${fileUrl}` && response.status() === 200);
  await link.click();
  const [reference, browserDownload] = await Promise.all([downloadResponsePromise, browserDownloadPromise]);
  assert.equal(reference.status(), 200);
  assert.match(reference.headers()["content-type"] ?? "", /^application\/pdf(?:;|$)/i);
  assert.match(browserDownload.suggestedFilename(), /\.pdf$/i);
  assert.deepEqual(await readFile(await browserDownload.path()), await readFile(expectedFile),
    "clicking the browser link downloads the stored PDF bytes");

  const crossTenant = await context.request.get(`${baseUrl}/tenant_documents/${documentId}/files/attachment?tenant_id=10`);
  assert.equal(crossTenant.status(), 404, "the same authenticated browser session cannot cross tenant scope");
  assert.deepEqual(pageErrors, [], `browser page errors: ${pageErrors.join(", ")}`);
  console.log("tenant attachment controls and authorized downloads passed in Chromium without JavaScript");
} finally {
  await browser.close();
}
