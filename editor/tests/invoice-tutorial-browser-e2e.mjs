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
  assert.match(css, /\.invoice-home-hero/);
  assert.match(css, /max-width: 991px/);
  assert.match(await page.locator("h1").innerText(), /Vom ersten Kunden bis zur fertigen Rechnung\.|From the first customer to the finished invoice\./);
  assert.equal(await page.locator(".invoice-home .invoice-workflow-card").count(), 3);
  assert.equal(await page.locator(".invoice-preview-number").innerText(), "RE-2026-001");
  assert.match(await page.locator(".invoice-preview").innerText(), /354,62 €|€354\.62/);
  const desktop = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
    cardLefts: [...document.querySelectorAll(".invoice-home .invoice-workflow-cards > article")].map((node) => Math.round(node.getBoundingClientRect().left)),
    invoiceButton: document.querySelector('a[href="/invoices/new"]') !== null,
  }));
  if (process.env.ZELYRA_INVOICE_SCREENSHOT) {
    await page.screenshot({ path: process.env.ZELYRA_INVOICE_SCREENSHOT, fullPage: true });
  }
  assert.ok(desktop.document <= desktop.viewport, `desktop page overflows: ${JSON.stringify(desktop)}`);
  assert.equal(new Set(desktop.cardLefts).size, 3, "desktop workflow should show three cards in one row");
  assert.equal(desktop.invoiceButton, true, "home page should offer invoice creation");
  for (const route of ["/customers", "/customers/new", "/items", "/items/new", "/invoices", "/invoices/new", "/invoice_lines", "/views/invoicedashboard"]) {
    await page.goto(`${baseUrl}${route}`, { waitUntil: "networkidle" });
    const width = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
    assert.ok(width.document <= width.viewport, `desktop CRUD route ${route} overflows: ${JSON.stringify(width)}`);
  }

  await page.goto(`${baseUrl}/invoices/new`, { waitUntil: "networkidle" });
  const dateField = page.locator('input[name="issued_at"]');
  assert.equal(await dateField.getAttribute("type"), "date");
  assert.match(await page.locator(".zelyra-field-hint").innerText(), /JJJJ-MM-TT|YYYY-MM-DD/);
  const invoiceNumber = page.locator('input[name="invoice_number"]');
  assert.equal(await invoiceNumber.getAttribute("readonly"), "");
  assert.match(await invoiceNumber.getAttribute("placeholder"), /automatisch|automatically/i);

  await page.goto(baseUrl, { waitUntil: "networkidle" });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(100);
  const mobile = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
    cardLefts: [...document.querySelectorAll(".invoice-home .invoice-workflow-cards > article")].map((node) => Math.round(node.getBoundingClientRect().left)),
    navCanScroll: document.querySelector(".tutorial-nav").scrollWidth >= document.querySelector(".tutorial-nav").clientWidth,
  }));
  assert.ok(mobile.document <= mobile.viewport, `mobile page overflows: ${JSON.stringify(mobile)}`);
  assert.equal(new Set(mobile.cardLefts).size, 1, "mobile workflow should use one column");
  assert.ok(mobile.navCanScroll, "mobile navigation must remain reachable");
  for (const route of ["/customers", "/customers/new", "/items", "/items/new", "/invoices", "/invoices/new", "/invoice_lines", "/views/invoicedashboard"]) {
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
