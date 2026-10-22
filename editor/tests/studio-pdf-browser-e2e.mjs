import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";
import axe from "axe-core";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-pdf-"));
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
  await writeFile(join(project, "main.zyl"), `view InvoicePreview {
    html {
        <!doctype html>
        <html><body><h1>Invoice preview</h1><slot /></body></html>
    }
}
fn main() {}
`);
  const port = await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const selected = server.address().port;
      server.close(() => resolvePort(selected));
    });
  });
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], { stdio: ["ignore", "pipe", "pipe"] });
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

  await page.locator('[data-studio-menu="run"]').click();
  await page.locator('[data-menu-target="pdf-preview-button"]').click();
  await page.waitForFunction(() => [...document.querySelector("#pdf-preview-view")?.options || []]
    .some((option) => option.value === "InvoicePreview"));
  await page.locator("#pdf-preview-data").fill("{invalid json");
  await page.locator("#pdf-preview-render").click();
  await page.waitForFunction(() => /JSON|JSON-Objekt|ungültig|invalid/i.test(document.querySelector("#pdf-preview-status")?.textContent || ""));

  await page.locator("#pdf-preview-data").fill("{}");
  const responsePromise = page.waitForResponse((response) => response.url().includes("/api/pdf-render") && response.request().method() === "POST");
  await page.locator("#pdf-preview-render").click();
  const response = await responsePromise;
  const responseBody = await response.text();
  assert.equal(response.status(), 200, responseBody);
  const pdf = JSON.parse(responseBody);
  const bytes = Buffer.from(pdf.base64, "base64");
  assert.equal(pdf.mimeType, "application/pdf");
  assert.equal(bytes.subarray(0, 5).toString("ascii"), "%PDF-");
  assert.equal(pdf.bytes, bytes.length);
  await page.waitForFunction(() => !document.querySelector("#pdf-preview-frame")?.hidden);
  assert.equal(await page.locator("#pdf-preview-download").isVisible(), true);
  assert.equal(await page.locator("#pdf-preview-download").getAttribute("download"), "zelyra-preview.pdf");
  const sameOrigin = new URL(url).origin;
  await page.route(`${sameOrigin}/__zelyra/axe-test.js`, (route) => route.fulfill({
    status: 200,
    contentType: "text/javascript",
    body: axe.source,
  }));
  await page.addScriptTag({ url: `${sameOrigin}/__zelyra/axe-test.js` });
  const accessibility = await page.evaluate(() => axe.run(document, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
  }));
  assert.deepEqual(
    accessibility.violations.map(({ id, impact, nodes }) => ({ id, impact, targets: nodes.map(({ target }) => target) })),
    [],
    "Studio PDF dialog must pass automated WCAG A/AA checks",
  );
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.locator("#pdf-preview-download").click(),
  ]);
  assert.equal(download.suggestedFilename(), "zelyra-preview.pdf");
  const downloaded = await readFile(await download.path());
  assert.equal(downloaded.subarray(0, 5).toString("ascii"), "%PDF-");
  assert.deepEqual(downloaded, bytes);
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio PDF preview rendered and downloaded the same real ${bytes.length}-byte PDF in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
