import assert from "node:assert/strict";
import axe from "axe-core";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-canvas-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
let processHandle;
let browser;

try {
  await writeFile(join(project, "main.zyl"), `database main { engine: mariadb }
component Banner {
    html { <aside><slot /></aside> }
}
view Shell {
    html { <html><body><main><Banner><slot /></Banner></main></body></html> }
}
page "/reports/{id}" {
    view: Shell
    input { term: String }
    html { <h1>{term}</h1><a href="/customers" id="canvas-link">Customers</a><script>document.body.dataset.executed = "true"</script> }
}
page "/live" {
    html { <h1>Live Zelyra app</h1> }
}
fn main() {}
`);
  await writeFile(join(project, ".env"), "DATABASE_URL=mariadb://zelyra:zelyra@127.0.0.1:1/zelyra_test\n");
  await writeFile(join(project, "zelyra.theme.css"), `:where(html[data-z-ui]) {
  --z-color-primary: #23654f;
  --z-color-on-primary: #ffffff;
}
`);
  const port = await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const selected = server.address().port;
      server.close(() => resolvePort(selected));
    });
  });
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], {
    env: { ...process.env, DATABASE_URL: "mariadb://zelyra:zelyra@127.0.0.1:1/zelyra_test" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const url = await new Promise((resolveUrl, reject) => {
    let output = "";
    processHandle.stdout.on("data", (chunk) => {
      output += chunk.toString();
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/);
      if (match) resolveUrl(match[0]);
    });
    processHandle.once("error", reject);
    processHandle.once("exit", (code) => reject(new Error(`Studio exited before startup: ${code}`)));
    setTimeout(() => reject(new Error("Studio startup timed out")), 10_000);
  });
  browser = await engine.launch({ headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator("#canvas-button").click();
  await page.waitForFunction(() => document.querySelector("#canvas-source")?.disabled === false);
  await page.locator("#canvas-target").selectOption({ label: "page · /reports/{id}" });
  await page.locator("#canvas-sample").fill('{"term":"<Acme & Co>"}');
  await page.locator("#canvas-render").click();
  await page.waitForFunction(() => document.querySelector("#canvas-source")?.disabled === false);
  assert.match(await page.locator("#canvas-status").innerText(), /Statische Canvas-Vorschau.*keine Projektfunktionen oder Skripte.*„Starten“.*„Anwendung im Browser öffnen“/);
  assert.equal(await page.locator("#canvas-frame").getAttribute("sandbox"), "", "static Canvas must remain isolated");
  const preview = page.frameLocator("#canvas-frame");
  await preview.locator("h1").waitFor();
  assert.equal(await preview.locator("h1").textContent(), "<Acme & Co>");
  assert.equal(await preview.locator("script").count(), 1, "preview keeps project markup visible for inspection");
  assert.equal(await preview.locator("body").getAttribute("data-executed"), null, "preview must not execute project scripts");
  assert.equal(await preview.locator("body").getAttribute("inert"), "", "static canvas content must not accept user interaction");
  const link = preview.locator("#canvas-link");
  await link.waitFor();
  const focused = await link.evaluate((element) => { element.focus(); return element.ownerDocument.activeElement === element; });
  assert.equal(focused, false, "static canvas links must not accept keyboard focus");
  const clickBlocked = await link.click({ timeout: 500 }).then(() => false, () => true);
  assert.equal(clickBlocked, true, "static canvas links must not navigate on pointer activation");
  assert.equal(await preview.locator("h1").textContent(), "<Acme & Co>", "static preview content remains visible after blocked interaction");
  await page.locator("#canvas-component-target").focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => document.querySelector(".cm-activeLine")?.textContent?.includes("component Banner"));
  assert.match(await page.locator(".cm-activeLine").textContent(), /component Banner/);
  await page.locator("#canvas-component-use-target").focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => document.querySelector(".cm-activeLine")?.textContent?.includes("<Banner>"));
  assert.match(await page.locator(".cm-activeLine").textContent(), /<Banner>/);

  const runResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/run") && response.request().method() === "POST",
  );
  await page.locator("#run-button").click();
  assert.equal((await runResponse).status(), 200, "the explicit Run action must start the real local web app");
  await page.locator("#run-output-web-link").waitFor({ state: "visible" });
  const appUrl = await page.locator("#run-output-web-url").getAttribute("href");
  assert.match(appUrl, /^http:\/\/127\.0\.0\.1:\d+$/);
  const appPage = await browser.newPage();
  await appPage.goto(`${appUrl}/live`, { waitUntil: "networkidle" });
  await appPage.getByRole("heading", { name: "Live Zelyra app" }).waitFor();
  assert.deepEqual(await appPage.locator("#canvas-status").count(), 0, "the live app opens outside the static Canvas iframe");
  await appPage.close();
  await page.locator("#run-output-close").click();
  await page.locator("#logs-button").click();
  await page.waitForFunction(() => /[1-9]\d*/.test(document.querySelector("#logs-status")?.textContent || ""));
  await page.waitForFunction(() => document.querySelector("#logs-content")?.textContent.includes("Zelyra server listening"));
  assert.match(await page.locator("#logs-content").innerText(), /INFO\s+preview\.stdout\nZelyra server listening on http:\/\/127\.0\.0\.1:/);
  await page.locator("#logs-close").click();
  const sessionToken = new URL(url).hash.slice(1);
  const stopStatus = await page.evaluate(async ({ sessionToken }) => {
    const response = await fetch("/__zelyra/editor/api/stop-web-preview", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Zelyra-Editor-Token": sessionToken,
      },
      body: "{}",
    });
    return response.status;
  }, { sessionToken });
  assert.equal(stopStatus, 200, "the test must stop the spawned preview");

  await page.locator("#outbox-button").click();
  await page.waitForFunction(() => document.querySelector("#outbox-status")?.textContent.includes("Outbox-Tabelle fehlt"));
  assert.match(await page.locator("#outbox-status").innerText(), /zelyra outbox setup main\.zyl.*Ohne Outbox-Nutzung/);
  await page.locator("#outbox-close").click();

  const primaryToken = await page.locator("#canvas-theme-token").evaluate((select) =>
    [...select.options].find((option) => option.textContent.includes("--z-color-primary"))?.value,
  );
  assert.notEqual(primaryToken, undefined, "theme editor should list the declared primary color");
  await page.locator("#canvas-theme-token").selectOption(primaryToken);
  await page.locator("#canvas-theme-color").evaluate((input) => {
    input.value = "#aabbcc";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await page.locator("#canvas-theme-apply").click();
  await page.waitForFunction(() => document.querySelector("#canvas-theme-status")?.textContent?.includes("geändert"));
  const themeSource = await page.locator(".cm-content").textContent();
  assert.match(themeSource, /--z-color-primary:\s*#aabbcc/);
  assert.match(themeSource, /--z-color-on-primary:\s*#000000/);
  await page.locator(".cm-content[contenteditable=true]").click();
  await page.keyboard.press("Control+z");
  assert.match(await page.locator(".cm-content").textContent(), /--z-color-primary:\s*#23654f/);
  assert.match(await page.locator(".cm-content").textContent(), /--z-color-on-primary:\s*#ffffff/);
  const sameOrigin = new URL(url).origin;
  await page.route(`${sameOrigin}/__zelyra/axe-test.js`, (route) => route.fulfill({
    status: 200, contentType: "text/javascript", body: axe.source,
  }));
  await page.addScriptTag({ url: `${sameOrigin}/__zelyra/axe-test.js` });
  const accessibility = await page.evaluate(async () => axe.run(document, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
  }));
  const violations = accessibility.violations.map(({ id, impact, nodes }) => ({
    id, impact, nodes: nodes.map((node) => ({ target: node.target, html: node.html, summary: node.failureSummary })),
  }));
  assert.deepEqual(violations, [], "Canvas controls must pass automated WCAG 2.2 A/AA checks");
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio Canvas rendered escaped sample data, blocked project scripts, and applied/undid theme edits in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
