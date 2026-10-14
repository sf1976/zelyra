import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const project = await mkdtemp(join(tmpdir(), "zelyra-runtime-browser-"));
const binary = resolve(process.env.ZELYRA_BIN || "target/debug/zelyra");
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"),
    '[project]\nname = "runtime-browser"\n[runtime]\nmax_stack_depth = 100\n');
  await writeFile(join(project, "main.zyl"),
    "fn direct() -> Int { return direct() }\nfn main() { loop { } }\n");
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
  const page = await browser.newPage({ viewport: { width: 360, height: 780 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator("#runtime-budget-button").click();
  await page.locator("#runtime-budget-dialog[open]").waitFor();
  await page.waitForFunction(() => document.querySelector("#runtime-stack-depth")?.textContent?.includes("100"));
  assert.match(await page.locator("#runtime-recursion-risks").textContent(), /direct/);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true,
    "the budget dialog must not overflow a 360px viewport");
  await page.locator("#runtime-budget-enabled").check();
  await page.locator("#runtime-budget-number").fill("30");
  const runResponse = page.waitForResponse((response) => response.url().endsWith("/api/run"), { timeout: 15_000 });
  await page.locator("#runtime-budget-run").click();
  const response = await runResponse;
  assert.equal(response.status(), 200);
  const result = await response.json();
  assert.equal(result.maxSteps, 30);
  assert.equal(result.budgetExceeded, true);
  assert.equal(result.timedOut, false);
  assert.ok(Number.isFinite(result.durationMs));
  await page.waitForFunction(() => document.querySelector("#run-output-state")?.textContent?.includes("Schrittbudget")
    || document.querySelector("#run-output-state")?.textContent?.includes("Step budget"));
  assert.match(await page.locator("#run-output-content").textContent(), /E-RUNTIME-001/);
  await page.locator("#run-output-close").click();
  await page.locator("#runtime-budget-button").click();
  await page.locator("#runtime-budget-enabled").uncheck();
  const cancelledResponse = page.waitForResponse((reply) => reply.url().endsWith("/api/run"), { timeout: 15_000 });
  await page.locator("#runtime-budget-run").click();
  await page.locator("#run-output-cancel:not([hidden])").waitFor();
  await page.waitForTimeout(100);
  await page.locator("#run-output-cancel").click();
  const cancelled = await (await cancelledResponse).json();
  assert.equal(cancelled.cancelled, true);
  assert.equal(cancelled.timedOut, false);
  assert.deepEqual(errors, []);
  console.log(`Studio runtime budget and recursion browser flow passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
