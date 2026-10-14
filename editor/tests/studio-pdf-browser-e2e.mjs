import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const project = await mkdtemp(join(tmpdir(), "zelyra-pdf-browser-"));
const binary = resolve(process.env.ZELYRA_BIN || "target/debug/zelyra");
const source = "view InvoicePrint { html { <h1>Invoice {number}</h1> } } fn main() { assert(false) }\n";
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "pdf-browser"\n');
  await writeFile(join(project, "main.zyl"), source);
  const port = await new Promise((resolvePort, reject) => {
    const socket = createServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const selected = socket.address().port;
      socket.close(() => resolvePort(selected));
    });
  });
  server = spawn(binary, ["editor", project, "--no-open", "--port", String(port)],
    { stdio: ["ignore", "pipe", "pipe"] });
  const url = await Promise.race([
    new Promise((resolveUrl, reject) => {
      let output = "";
      server.stdout.on("data", (chunk) => {
        output += chunk.toString();
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/);
        if (match) resolveUrl(match[0]);
      });
      server.once("exit", (code) => reject(new Error(`Studio exited: ${code}`)));
    }),
    new Promise((_, reject) => { startupTimer = setTimeout(() => reject(new Error("Studio startup timed out")), 10_000); }),
  ]);
  clearTimeout(startupTimer);
  browser = await engine.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator("#pdf-preview-button").click();
  await page.locator("#pdf-preview-view option").first().waitFor({ state: "attached" });
  assert.equal(await page.locator("#pdf-preview-view").inputValue(), "InvoicePrint");
  await page.locator("#pdf-preview-data").fill('{"number":"R-104"}');
  await page.locator("#pdf-preview-render").click();
  await page.locator("#pdf-preview-download:not([hidden])").waitFor();
  const pdf = await page.evaluate(async () => {
    const url = document.querySelector("#pdf-preview-download").href;
    const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
    return { header: new TextDecoder().decode(bytes.slice(0, 8)), size: bytes.length };
  });
  assert.match(pdf.header, /^%PDF-1\.4/);
  assert.ok(pdf.size > 500);
  assert.match(await page.locator("#pdf-preview-status").textContent(), /PDF/);
  assert.equal(await readFile(join(project, "main.zyl"), "utf8"), source);
  assert.deepEqual(errors, []);
  console.log(`Studio real PDF preview passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
