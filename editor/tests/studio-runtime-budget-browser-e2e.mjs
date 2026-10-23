import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";
import axe from "axe-core";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-budget-"));
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
  await writeFile(join(project, "main.zyl"), `fn main() {
    print("Studio output is visible")
    while true {}
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
  await page.locator('.studio-menu-popover [data-menu-target="runtime-budget-button"]').click();
  await page.waitForFunction(() => Boolean(document.querySelector("#runtime-stack-depth")?.textContent));
  const backend = page.locator("#run-backend");
  assert.equal(await backend.inputValue(), "ast", "AST remains the default backend");
  await backend.selectOption("mir");
  assert.equal(await page.evaluate(() => localStorage.getItem("zelyra-studio-run-backend")), "mir");
  await page.locator("#runtime-budget-enabled").check();
  await page.locator("#runtime-budget-number").fill("1000");
  await page.locator("#runtime-budget-run").click();
  await page.locator("#run-output[open]").waitFor();
  await page.waitForFunction(() => /budget|Schrittbudget/i.test(document.querySelector("#run-output-state")?.textContent || ""));
  assert.match(await page.locator("#run-output-state").textContent(), /Schrittbudget überschritten|Step budget exceeded/);
  const output = await page.locator("#run-output-content").textContent();
  assert.match(output, /Studio output is visible/);
  assert.match(output, /E-RUNTIME-001|execution step budget exceeded/);
  assert.equal(await page.locator("#run-output-state").getAttribute("data-kind"), "error");

  const sameOrigin = new URL(url).origin;
  await page.route(`${sameOrigin}/__zelyra/axe-test.js`, (route) => route.fulfill({
    status: 200, contentType: "text/javascript", body: axe.source,
  }));
  await page.addScriptTag({ url: `${sameOrigin}/__zelyra/axe-test.js` });
  const accessibility = await page.evaluate(() => axe.run(document, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
  }));
  assert.deepEqual(accessibility.violations.map(({ id, nodes }) => ({ id, targets: nodes.map(({ target }) => target) })), []);
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio displayed real program output and stopped at the 1,000-step budget in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
