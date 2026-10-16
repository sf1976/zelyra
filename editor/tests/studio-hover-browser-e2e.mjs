import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";
import axe from "axe-core";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const project = await mkdtemp(join(tmpdir(), "zelyra-studio-hover-"));
let processHandle;
let browser;

try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "studio-hover"\nversion = "1.0.0"\n');
  await writeFile(join(project, "main.zyl"), 'fn greet(name: String) -> String { message = name return message }\nfn main() { print(greet("Ada")) }\n');
  const port = await new Promise((resolvePort, reject) => {
    const socket = createServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const value = socket.address().port;
      socket.close(() => resolvePort(value));
    });
  });
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], { stdio: ["ignore", "pipe", "pipe"] });
  let startupTimeout;
  const url = await Promise.race([
    new Promise((resolveUrl, reject) => {
      let output = "";
      processHandle.stdout.on("data", (chunk) => {
        output += chunk.toString();
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/);
        if (match) resolveUrl(match[0]);
      });
      processHandle.once("exit", (code) => reject(new Error(`Studio exited ${code}: ${output}`)));
    }),
    new Promise((_, reject) => {
      startupTimeout = setTimeout(() => reject(new Error("Studio startup timed out")), 10_000);
    }),
  ]);
  clearTimeout(startupTimeout);

  browser = await engine.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  page.setDefaultTimeout(10_000);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content").waitFor();
  await page.getByText("return message", { exact: false }).first().waitFor();
  const point = await page.evaluate(() => {
    const line = [...document.querySelectorAll(".cm-line")].find((node) => node.textContent.includes("return message"));
    const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const text = walker.currentNode;
      const offset = text.textContent.indexOf("message");
      if (offset < 0) continue;
      const range = document.createRange();
      range.setStart(text, offset + 2);
      range.setEnd(text, offset + 3);
      const rect = range.getBoundingClientRect();
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    }
    throw new Error("local symbol text node not found");
  });
  await page.mouse.move(point.x, point.y);
  const tooltip = page.locator(".cm-tooltip-zelyra-hover");
  await tooltip.waitFor();
  assert.equal(await tooltip.textContent(), "local message: String");
  const origin = new URL(url).origin;
  await page.route(`${origin}/__zelyra/axe-test.js`, (route) => route.fulfill({
    status: 200,
    contentType: "text/javascript",
    body: axe.source,
  }));
  await page.addScriptTag({ url: `${origin}/__zelyra/axe-test.js` });
  const accessibility = await page.evaluate(async () => axe.run(document, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
  }));
  assert.deepEqual(accessibility.violations.map(({ id }) => id), [], "hover tooltip must pass Axe WCAG 2.2 A/AA checks");
  await page.mouse.click(point.x, point.y);
  await page.keyboard.press("Alt+Shift+h");
  await tooltip.waitFor();
  assert.equal(await tooltip.textContent(), "local message: String");
  await page.keyboard.press("Escape");
  await tooltip.waitFor({ state: "detached" });
  console.log(`Compiler-backed Studio hover passed with mouse and keyboard in ${engineName}`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
