import assert from "node:assert/strict";
import { chromium } from "playwright";

const baseUrl = process.env.ZELYRA_INVOICE_E2E_URL;
assert.ok(baseUrl, "ZELYRA_INVOICE_E2E_URL must point to the running tutorial");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
  const externalRequests = [];
  page.on("request", (request) => {
    if (!request.url().startsWith(baseUrl)) externalRequests.push(request.url());
  });
  const stylesheet = page.waitForResponse((response) => response.url().endsWith("/__zelyra/theme.css"));
  await page.goto(baseUrl, { waitUntil: "networkidle" });
  const cssResponse = await stylesheet;
  assert.equal(cssResponse.status(), 200);
  const css = await cssResponse.text();
  assert.match(css, /Tabler v1\.6\.1/);
  assert.match(css, /\.tutorial-metric/);
  assert.match(css, /max-width: 991px/);
  assert.match(await page.locator("h1").innerText(), /Guten Tag im Rechnungsbüro/);
  assert.equal(await page.locator(".tutorial-metric").count(), 4);
  const desktop = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
    stepsColumns: getComputedStyle(document.querySelector(".tutorial-steps")).gridTemplateColumns,
    dashboardColumns: [...document.querySelectorAll(".tutorial-metric")].map((node) => node.getBoundingClientRect().left),
    headerTitleLeft: document.querySelector(".tutorial-page-header > div").getBoundingClientRect().left,
    headerActionsLeft: document.querySelector(".tutorial-header-actions").getBoundingClientRect().left,
    customerCount: Number(document.querySelector(".tutorial-metric strong").textContent),
  }));
  if (process.env.ZELYRA_INVOICE_SCREENSHOT) {
    await page.screenshot({ path: process.env.ZELYRA_INVOICE_SCREENSHOT, fullPage: true });
  }
  assert.ok(desktop.document <= desktop.viewport, `desktop page overflows: ${JSON.stringify(desktop)}`);
  assert.equal(new Set(desktop.dashboardColumns).size, 4, `desktop metrics should form four columns: ${JSON.stringify(desktop)}`);
  assert.equal(desktop.stepsColumns.split(" ").length, 2, "desktop workflow should use two columns");
  assert.ok(desktop.headerTitleLeft < desktop.headerActionsLeft, "desktop page title should align before the actions");
  assert.ok(desktop.customerCount >= 1, "dashboard should display persisted customer records");
  for (const route of ["/customers", "/customers/new", "/items", "/items/new", "/invoices", "/invoices/new", "/invoice_lines"]) {
    await page.goto(`${baseUrl}${route}`, { waitUntil: "networkidle" });
    const width = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
    assert.ok(width.document <= width.viewport, `desktop CRUD route ${route} overflows: ${JSON.stringify(width)}`);
  }

  await page.goto(baseUrl, { waitUntil: "networkidle" });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(100);
  const mobile = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
    stepsColumns: getComputedStyle(document.querySelector(".tutorial-steps")).gridTemplateColumns,
    navCanScroll: document.querySelector(".tutorial-nav").scrollWidth >= document.querySelector(".tutorial-nav").clientWidth,
  }));
  assert.ok(mobile.document <= mobile.viewport, `mobile page overflows: ${JSON.stringify(mobile)}`);
  assert.equal(mobile.stepsColumns.split(" ").length, 1, "mobile workflow should use one column");
  assert.ok(mobile.navCanScroll, "mobile navigation must remain reachable");
  for (const route of ["/customers", "/customers/new", "/items", "/items/new", "/invoices", "/invoices/new", "/invoice_lines"]) {
    await page.goto(`${baseUrl}${route}`, { waitUntil: "networkidle" });
    const width = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
    assert.ok(width.document <= width.viewport, `mobile CRUD route ${route} overflows: ${JSON.stringify(width)}`);
  }

  await page.goto(baseUrl, { waitUntil: "networkidle" });
  await page.emulateMedia({ media: "print" });
  assert.equal(await page.locator(".tutorial-navbar").evaluate((node) => getComputedStyle(node).display), "none");
  assert.deepEqual(externalRequests, [], "tutorial must not request remote assets");
  console.log("invoice tutorial desktop, mobile, print, and offline styling checks passed");
} finally {
  await browser.close();
}
