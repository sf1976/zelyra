import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { chromium, firefox, webkit } from "playwright";
import axe from "axe-core";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

const baseUrl = process.env.ZELYRA_INVOICE_E2E_URL;
assert.ok(baseUrl, "ZELYRA_INVOICE_E2E_URL must point to the running tutorial");

async function assertVisualSnapshot(page, name) {
  const viewport = page.viewportSize();
  await page.evaluate(async () => {
    if (history.scrollRestoration) history.scrollRestoration = "manual";
    await document.fonts.ready;
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    window.scrollTo({ top: 0, left: 0, behavior: "instant" });
    await new Promise((resolve) => requestAnimationFrame(resolve));
  });
  await page.waitForTimeout(120);
  const documentHeight = await page.evaluate(() => document.documentElement.scrollHeight);
  await page.setViewportSize({ width: viewport.width, height: Math.max(viewport.height, documentHeight) });
  await page.evaluate(() => window.scrollTo({ top: 0, left: 0, behavior: "instant" }));
  const actualBuffer = await page.screenshot({ animations: "disabled" });
  await page.setViewportSize(viewport);
  const baselinePath = new URL(`./visual-baselines/${name}.png`, import.meta.url);
  if (process.env.ZELYRA_UPDATE_VISUAL_BASELINES === "1") {
    await mkdir(new URL("./visual-baselines/", import.meta.url), { recursive: true });
    await writeFile(baselinePath, actualBuffer);
    return;
  }
  const baseline = PNG.sync.read(await readFile(baselinePath));
  const actual = PNG.sync.read(actualBuffer);
  assert.deepEqual(
    { width: actual.width, height: actual.height },
    { width: baseline.width, height: baseline.height },
    `${name} screenshot dimensions changed`,
  );
  const diff = new PNG({ width: actual.width, height: actual.height });
  // Allow small cross-platform font rasterization differences while keeping
  // the page-wide changed-pixel budget strict for layout and color regressions.
  const changedPixels = pixelmatch(baseline.data, actual.data, diff.data, actual.width, actual.height, { threshold: 0.2 });
  const allowedPixels = Math.ceil(actual.width * actual.height * 0.002);
  if (changedPixels > allowedPixels) {
    const diffPath = new URL(`./visual-baselines/${name}.actual-diff.png`, import.meta.url);
    await writeFile(diffPath, PNG.sync.write(diff));
    await writeFile(new URL(`./visual-baselines/${name}.actual.png`, import.meta.url), actualBuffer);
  }
  assert.ok(changedPixels <= allowedPixels, `${name} visual regression: ${changedPixels} changed pixels, limit ${allowedPixels}`);
}

const browserName = process.env.ZELYRA_BROWSER || "chromium";
const browserType = { chromium, firefox, webkit }[browserName];
assert.ok(browserType, `unsupported ZELYRA_BROWSER: ${browserName}`);
const browser = await browserType.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.addInitScript(() => {
    window.__zelyraPrintCalls = 0;
    window.print = () => { window.__zelyraPrintCalls += 1; };
  });
  const externalRequests = [];
  page.on("request", (request) => {
    if (!request.url().startsWith(baseUrl)) externalRequests.push(request.url());
  });
  const stylesheet = page.waitForResponse((response) => response.url().endsWith("/__zelyra/theme.css"));
  await page.goto(baseUrl, { waitUntil: "networkidle" });
  const cssResponse = await stylesheet;
  assert.equal(cssResponse.status(), 200);
  const css = await cssResponse.text();
  assert.match(css, /Invoice tutorial theme: local overrides of Zelyra UI/);
  assert.match(css, /--z-color-primary/);
  assert.match(css, /\.invoice-home-hero/);
  assert.match(css, /invoice-ui-gallery \.z-toast-region/);
  assert.match(css, /max-width: 991px/);
  assert.match(await page.locator("h1").innerText(), /Vom ersten Kunden bis zur fertigen Rechnung\.|From the first customer to the finished invoice\./);
  assert.equal(await page.locator(".invoice-home .invoice-workflow-card").count(), 3);
  assert.equal(await page.locator(".invoice-preview-number").innerText(), "RE-2026-001");
  assert.match(await page.locator(".invoice-preview").innerText(), /354,62 €|€354\.62/);
  const desktop = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
    cardLefts: [...document.querySelectorAll(".invoice-home .invoice-workflow .z-grid > article")].map((node) => Math.round(node.getBoundingClientRect().left)),
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
    const width = await page.evaluate(() => ({
      viewport: innerWidth,
      document: document.documentElement.scrollWidth,
      body: { client: document.body.clientWidth, scroll: document.body.scrollWidth },
      offenders: [...document.querySelectorAll("html, body, body *")]
        .map((element) => ({
          tag: element.tagName.toLowerCase(),
          className: typeof element.className === "string" ? element.className : "",
          text: element.children.length ? "" : element.textContent.trim().slice(0, 80),
          right: element.getBoundingClientRect().right,
          width: element.getBoundingClientRect().width,
          client: element.clientWidth,
          scroll: element.scrollWidth,
          before: getComputedStyle(element, "::before").content,
          after: getComputedStyle(element, "::after").content,
          font: getComputedStyle(element).font,
        }))
        .filter((element) => element.right > innerWidth || element.scroll > element.client)
        .slice(0, 8),
    }));
    assert.ok(width.document <= width.viewport, `desktop CRUD route ${route} overflows: ${JSON.stringify(width)}`);
  }

  await page.goto(`${baseUrl}/views/invoicedashboard`, { waitUntil: "networkidle" });
  const dashboardLayout = await page.evaluate(() => {
    const bounds = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return null;
      const rectangle = element.getBoundingClientRect();
      return { top: rectangle.top, left: rectangle.left, right: rectangle.right, width: rectangle.width };
    };
    return {
      main: bounds("#invoice-app-main"),
      sidebar: bounds(".z-app-shell__sidebar"),
      page: bounds("#invoice-app-main .tutorial-content > .z-generated-page"),
      legend: bounds("#invoice-app-main .zelyra-query-controls legend"),
      fields: bounds("#invoice-app-main .z-query-controls__fields"),
      nestedMainCount: document.querySelectorAll("#invoice-app-main main").length,
      tableClass: document.querySelector("#invoice-app-main table")?.className,
      tableDisplay: getComputedStyle(document.querySelector("#invoice-app-main table")).display,
      formInputClass: document.querySelector("#invoice-app-main input[name=search]")?.className,
      visibleHeaders: [...document.querySelectorAll("#invoice-app-main .z-table thead th")].map((cell) => cell.innerText),
      sortOptions: [...document.querySelectorAll("#invoice-app-main select[name=sort] option")].map((option) => ({ value: option.value, label: option.innerText })),
    };
  });
  assert.ok(dashboardLayout.main && dashboardLayout.sidebar && dashboardLayout.page, `invoice overview layout is incomplete: ${JSON.stringify(dashboardLayout)}`);
  assert.ok(dashboardLayout.legend && dashboardLayout.fields, `invoice overview query controls are incomplete: ${JSON.stringify(dashboardLayout)}`);
  assert.equal(dashboardLayout.nestedMainCount, 0, `generated overview must not nest a second main landmark: ${JSON.stringify(dashboardLayout)}`);
  assert.ok(dashboardLayout.page.top < dashboardLayout.main.top + 100, `invoice rows must begin in the main content area: ${JSON.stringify(dashboardLayout)}`);
  assert.ok(dashboardLayout.main.left >= dashboardLayout.sidebar.right - 1, `invoice overview must sit to the right of the sidebar: ${JSON.stringify(dashboardLayout)}`);
  assert.ok(dashboardLayout.legend.top < dashboardLayout.fields.top, `query controls legend must stay above its fields: ${JSON.stringify(dashboardLayout)}`);
  assert.equal(dashboardLayout.tableClass, "z-table");
  assert.equal(dashboardLayout.tableDisplay, "table", "tutorial theme must preserve native Zelyra table layout");
  assert.equal(dashboardLayout.formInputClass, "z-input");
  assert.ok(dashboardLayout.visibleHeaders.some((header) => /Customer|Kunde/.test(header)));
  assert.ok(dashboardLayout.visibleHeaders.some((header) => /Net total|Nettosumme/.test(header)));
  assert.ok(dashboardLayout.sortOptions.some(({ value, label }) => value === "customer_name" && /Customer|Kunde/.test(label)));
  if (browserName === "chromium") await assertVisualSnapshot(page, "invoice-dashboard-desktop");

  await page.goto(`${baseUrl}/invoices/new`, { waitUntil: "networkidle" });
  assert.match(await page.locator("h1").innerText(), /Create Invoice|Rechnung anlegen/);
  const createFormStyle = await page.locator(".zelyra-crud-form-card").evaluate((card) => ({
    gridColumns: getComputedStyle(card.querySelector("form")).gridTemplateColumns,
    submitClass: card.querySelector('button[type="submit"]').className,
    background: getComputedStyle(card).backgroundImage,
  }));
  assert.equal(createFormStyle.gridColumns.split(" ").length, 2, `desktop create form should group its fields into two columns: ${JSON.stringify(createFormStyle)}`);
  assert.match(createFormStyle.submitClass, /z-button--primary/);
  assert.notEqual(createFormStyle.background, "none");
  if (browserName === "chromium") await assertVisualSnapshot(page, "invoice-create-form-desktop");

  const galleryResponse = await page.goto(`${baseUrl}/ui/components`, { waitUntil: "networkidle" });
  const chartCount = await page.locator(".z-chart").count();
  assert.equal(chartCount, 3, `the overview should include bar, line, and donut charts (HTTP ${galleryResponse.status()}): ${(await page.locator("body").innerText()).slice(0, 700)}`);
  assert.equal(await page.locator(".invoice-ui-gallery table caption").count(), 2, "chart/table values should have text alternatives and captions");
  assert.equal(await page.locator('input[type="date"]').count(), 1);
  assert.equal(await page.locator('input[type="number"]').count(), 1);
  assert.equal(await page.locator('input[type="email"][aria-invalid="true"]').count(), 1);
  assert.equal(await page.locator('input[role="switch"]').count(), 1);
  assert.equal(await page.locator('#ui-reminder').count(), 1, `switch id missing: ${(await page.locator("body").innerText()).slice(0, 400)}`);
  assert.ok((await page.locator('#ui-reminder').getAttribute("class")).includes("z-switch__input--warning"));
  assert.equal(await page.locator('input[type="radio"].z-check--success').count(), 1);
  assert.equal(await page.locator('.z-badge[class*="z-badge--"]').count(), 5);
  await page.addScriptTag({ content: axe.source });
  const accessibility = await page.evaluate(async () => axe.run(document, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
  }));
  assert.deepEqual(
    accessibility.violations.map(({ id, impact, description, nodes }) => ({
      id,
      impact,
      description,
      nodes: nodes.map(({ target, failureSummary }) => ({ target, failureSummary })),
    })),
    [],
    "axe-core found WCAG A/AA violations in the component gallery",
  );
  const firstTab = page.getByRole("tab", { name: "Zusammenfassung" });
  const secondTab = page.getByRole("tab", { name: "Hinweise" });
  await firstTab.waitFor();
  assert.equal(await firstTab.getAttribute("aria-selected"), "true");
  await firstTab.focus();
  await page.keyboard.press("ArrowRight");
  assert.equal(await secondTab.getAttribute("aria-selected"), "true");
  assert.match(await page.getByRole("tabpanel").innerText(), /Sprunglinks und beide Inhalte (?:bleiben )?sichtbar/);
  const openDialog = page.getByRole("button", { name: "Dialog öffnen" });
  const dialog = page.locator("dialog[data-z-dialog]");
  const dialogId = await dialog.getAttribute("id");
  assert.match(dialogId, /^z-dialog-auto-/);
  assert.equal(await openDialog.getAttribute("aria-controls"), dialogId);
  assert.equal(await dialog.getAttribute("aria-labelledby"), `${dialogId}-title`);
  assert.equal(await openDialog.getAttribute("type"), "button");
  await page.evaluate((id) => {
    const opener = document.querySelector(`[data-z-dialog-open="${id}"]`);
    const dialog = document.getElementById(id);
    const placeholder = document.createComment("restore dialog opener");
    opener.before(placeholder);
    const form = document.createElement("form");
    form.id = "dialog-opener-form-fixture";
    form.dataset.submissions = "0";
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      form.dataset.submissions = String(Number(form.dataset.submissions) + 1);
    });
    form.append(opener);
    document.body.append(form);
    const autofocus = document.createElement("button");
    autofocus.type = "button";
    autofocus.autofocus = true;
    autofocus.textContent = "Autofocus fixture";
    dialog.querySelector(".z-dialog__body").append(autofocus);
  }, dialogId);
  await openDialog.click();
  const dialogState = await dialog.evaluate((element) => ({
    open: element.open,
    ready: element.dataset.zReady,
    enhanced: element.dataset.zEnhanced,
    opener: document.querySelector(`[data-z-dialog-open="${element.id}"]`)?.dataset.zReady,
    focused: document.activeElement?.textContent.trim(),
  }));
  assert.equal(dialogState.open, true, `dialog did not open: ${JSON.stringify({ dialogState, pageErrors })}`);
  assert.equal(dialogState.focused, "Autofocus fixture", "native autofocus takes precedence over the close button");
  assert.equal(
    await page.evaluate((id) => {
      const opener = document.querySelector(`[data-z-dialog-open="${id}"]`);
      const dialog = document.getElementById(id);
      opener.focus();
      return dialog.contains(document.activeElement);
    }, dialogId),
    true,
    "the modal dialog keeps the background inert to programmatic focus",
  );
  await page.getByRole("button", { name: "Schließen" }).waitFor();
  await page.keyboard.press("Tab");
  const tabState = await page.evaluate(() => ({
    activeTag: document.activeElement?.tagName,
    activeId: document.activeElement?.id,
    insideDialog: Boolean(document.activeElement.closest("dialog")),
    dialogOpen: document.querySelector("dialog[data-z-dialog]")?.open,
  }));
  assert.equal(tabState.insideDialog, true, `native modal keyboard navigation remains inside the dialog: ${JSON.stringify(tabState)}`);
  await page.keyboard.press("Escape");
  await dialog.waitFor({ state: "hidden" });
  assert.equal(await openDialog.evaluate((element) => element === document.activeElement), true, "dialog close returns focus to its opener");
  assert.equal(await page.locator("#dialog-opener-form-fixture").getAttribute("data-submissions"), "0", "dialog opener does not submit its containing form");
  await page.evaluate(() => {
    const form = document.querySelector("#dialog-opener-form-fixture");
    const opener = form.querySelector("[data-z-dialog-open]");
    const placeholder = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
    let marker;
    while ((marker = placeholder.nextNode())) {
      if (marker.data === "restore dialog opener") break;
    }
    marker.replaceWith(opener);
    form.remove();
    document.querySelector("dialog[data-z-dialog] .z-dialog__body button[autofocus]").remove();
  });
  await page.setViewportSize({ width: 360, height: 700 });
  await dialog.evaluate((element) => {
    const body = element.querySelector(".z-dialog__body");
    for (let index = 0; index < 30; index += 1) {
      const paragraph = document.createElement("p");
      paragraph.dataset.dialogTestContent = "true";
      paragraph.textContent = `Long dialog content ${index}`;
      body.append(paragraph);
    }
  });
  await openDialog.click();
  const narrowDialog = await dialog.evaluate((element) => ({
    width: element.getBoundingClientRect().width,
    viewport: innerWidth,
    scrollHeight: element.querySelector(".z-dialog__body").scrollHeight,
    clientHeight: element.querySelector(".z-dialog__body").clientHeight,
  }));
  assert.ok(narrowDialog.width <= narrowDialog.viewport, `dialog exceeds a narrow viewport: ${JSON.stringify(narrowDialog)}`);
  assert.ok(narrowDialog.scrollHeight > narrowDialog.clientHeight, `long dialog content should scroll inside the body: ${JSON.stringify(narrowDialog)}`);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1440, height: 960 });
  await dialog.evaluate((element) => element.querySelectorAll("p[data-dialog-test-content]").forEach((paragraph) => paragraph.remove()));
  await page.getByRole("button", { name: "Komponentenübersicht drucken" }).click();
  assert.equal(await page.evaluate(() => window.__zelyraPrintCalls), 1, "PrintAction opens the native print flow");
  assert.equal(
    await page.getByRole("link", { name: "E-Mail-Entwurf öffnen" }).getAttribute("href"),
    "mailto:rechnung%40example.test?subject=RE%202026%26001&body=Rechnung%20RE%202026%26001",
    "EmailLink URL-encodes recipient, subject, and draft body so they cannot add mailto parameters",
  );
  await page.evaluate(() => window.ZelyraUI.setTheme("dark"));
  assert.equal(await page.locator("html").getAttribute("data-z-theme"), "dark");
  await page.evaluate(() => window.ZelyraUI.setTheme("light"));
  assert.equal(await page.locator("html").getAttribute("data-z-theme"), "light");
  await page.evaluate(() => window.ZelyraUI.setDensity("compact"));
  assert.equal(await page.locator("html").getAttribute("data-z-density"), "compact");
  await page.evaluate(() => window.ZelyraUI.setDensity("comfortable"));
  assert.equal(await page.locator("html").getAttribute("data-z-density"), null);
  await page.setViewportSize({ width: 1440, height: 960 });
  await firstTab.click();
  if (browserName === "chromium") await assertVisualSnapshot(page, "invoice-ui-gallery-desktop");
  for (const width of [360, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    const size = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
    assert.ok(size.document <= size.viewport, `component gallery overflows at ${width}px: ${JSON.stringify(size)}`);
  }
  // Desktop browser zoom to 200% exposes a narrower CSS layout viewport.
  // Headless Chromium cannot control browser chrome zoom, so exercise 360 CSS
  // pixels and keep the visual snapshot at the real mobile device scale.
  await page.setViewportSize({ width: 360, height: 900 });
  if (browserName === "chromium") await assertVisualSnapshot(page, "invoice-ui-gallery-mobile");
  const zoomed = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
  assert.ok(zoomed.document <= zoomed.viewport, `component gallery overflows at the 360 CSS-pixel layout viewport used by 200% zoom: ${JSON.stringify(zoomed)}`);

  const noJsPage = await browser.newPage({
    viewport: { width: 390, height: 844 },
    javaScriptEnabled: false,
  });
  await noJsPage.goto(`${baseUrl}/ui/components`, { waitUntil: "networkidle" });
  assert.equal(await noJsPage.locator("#invoice-app-navigation").isVisible(), true, "navigation stays visible without JavaScript");
  assert.equal(await noJsPage.getByRole("dialog", { name: "Aktion prüfen" }).isVisible(), true, "dialog content stays available without JavaScript");
  assert.equal(await noJsPage.locator(".z-tabs__panel:visible").count(), 2, "tab contents stay visible without JavaScript");
  assert.equal(await noJsPage.getByRole("button", { name: "Komponentenübersicht drucken" }).isVisible(), true);
  await noJsPage.close();

  await page.goto(`${baseUrl}/invoices/new`, { waitUntil: "networkidle" });
  const dateField = page.locator('input[name="issued_at"]');
  assert.equal(await dateField.getAttribute("type"), "date");
  assert.match(await page.locator(".zelyra-field-hint").innerText(), /JJJJ-MM-TT|YYYY-MM-DD/);
  const invoiceNumber = page.locator('input[name="invoice_number"]');
  assert.equal(await invoiceNumber.getAttribute("readonly"), "");
  assert.match(await invoiceNumber.getAttribute("placeholder"), /automatisch|automatically/i);

  await page.goto(baseUrl, { waitUntil: "networkidle" });
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.evaluate(() => window.ZelyraUI.setTheme("light", false));
  await page.emulateMedia({ colorScheme: "light" });
  if (browserName === "chromium") await assertVisualSnapshot(page, "invoice-home-desktop");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForFunction(() => document.querySelector(".z-app-shell__navigation")?.dataset.zCollapsed === "true");
  await page.waitForTimeout(100);
  const mobile = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
    cardLefts: [...document.querySelectorAll(".invoice-home .invoice-workflow .z-grid > article")].map((node) => Math.round(node.getBoundingClientRect().left)),
    navExpanded: document.querySelector(".z-app-shell__navigation").dataset.zCollapsed !== "true",
    navToggleHidden: document.querySelector("[data-z-mobile-nav-toggle]").hidden,
    navToggleExpanded: document.querySelector("[data-z-mobile-nav-toggle]").getAttribute("aria-expanded"),
    navCompact: document.querySelector("[data-z-mobile-nav-toggle]").dataset.zNavCompact,
    compactMedia: window.matchMedia("(max-width: 600px)").matches,
  }));
  assert.ok(mobile.document <= mobile.viewport, `mobile page overflows: ${JSON.stringify(mobile)}`);
  assert.equal(new Set(mobile.cardLefts).size, 1, "mobile workflow should use one column");
  assert.equal(mobile.navExpanded, false, `mobile navigation starts collapsed after enhancement: ${JSON.stringify(mobile)}`);
  assert.equal(mobile.navToggleHidden, false, "mobile navigation has a visible toggle");
  await page.locator("[data-z-mobile-nav-toggle]").click();
  assert.equal(await page.locator("[data-z-mobile-nav-toggle]").getAttribute("aria-expanded"), "true");
  for (const route of ["/customers", "/customers/new", "/items", "/items/new", "/invoices", "/invoices/new", "/invoice_lines", "/views/invoicedashboard"]) {
    await page.goto(`${baseUrl}${route}`, { waitUntil: "networkidle" });
    const width = await page.evaluate(() => ({
      viewport: innerWidth,
      document: document.documentElement.scrollWidth,
      body: { client: document.body.clientWidth, scroll: document.body.scrollWidth },
      offenders: [...document.querySelectorAll("html, body, body *")]
        .map((element) => ({
          tag: element.tagName.toLowerCase(),
          className: typeof element.className === "string" ? element.className : "",
          text: element.children.length ? "" : element.textContent.trim().slice(0, 80),
          right: element.getBoundingClientRect().right,
          width: element.getBoundingClientRect().width,
          client: element.clientWidth,
          scroll: element.scrollWidth,
          before: getComputedStyle(element, "::before").content,
          after: getComputedStyle(element, "::after").content,
          font: getComputedStyle(element).font,
        }))
        .filter((element) => element.right > innerWidth || element.scroll > element.client)
        .slice(0, 8),
    }));
    assert.ok(width.document <= width.viewport, `mobile CRUD route ${route} overflows: ${JSON.stringify(width)}`);
  }
  await page.goto(`${baseUrl}/invoices/new`, { waitUntil: "networkidle" });
  const mobileFormColumns = await page.locator(".zelyra-crud-form-card form").evaluate((form) => getComputedStyle(form).gridTemplateColumns.split(" ").length);
  assert.equal(mobileFormColumns, 1, "create form fields become one column on a phone");

  const generatedNavigation = page.locator("[data-z-mobile-nav-toggle]");
  await generatedNavigation.waitFor({ state: "visible" });
  assert.equal(await generatedNavigation.getAttribute("aria-expanded"), "false");
  await generatedNavigation.click();
  assert.equal(await generatedNavigation.getAttribute("aria-expanded"), "true");
  await generatedNavigation.focus();
  await page.keyboard.press("Escape");
  assert.equal(await generatedNavigation.getAttribute("aria-expanded"), "false", "Escape closes mobile navigation");
  assert.equal(await generatedNavigation.evaluate((element) => element === document.activeElement), true, "Escape returns focus to the navigation control");

  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto(baseUrl, { waitUntil: "networkidle" });
  await page.emulateMedia({ media: "print" });
  assert.equal(await page.locator(".z-app-shell__header").evaluate((node) => getComputedStyle(node).display), "none");
  await page.goto(`${baseUrl}/invoices/RE-2026-001/print`, { waitUntil: "networkidle" });
  await page.emulateMedia({ media: "print" });
  assert.equal(await page.locator(".zelyra-learning-assistant").evaluate((node) => getComputedStyle(node).display), "none", "learning assistant must not appear on printed invoices");
  assert.match(await page.locator(".invoice-print-document tbody").innerText(), /125\.00 €|125,00 €/);
  assert.equal(await page.locator(".invoice-print-document table").evaluate((node) => getComputedStyle(node).tableLayout), "fixed");
  if (browserName === "chromium") await assertVisualSnapshot(page, "invoice-print");
  assert.deepEqual(externalRequests, [], "tutorial must not request remote assets");
  console.log("invoice tutorial desktop, mobile, print, and offline styling checks passed");
} finally {
  await browser.close();
}
