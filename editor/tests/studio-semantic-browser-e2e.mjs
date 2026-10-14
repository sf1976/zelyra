import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-semantic-"));
const main = 'import "src/math.zyl" as math\nfn main() { print(math::answer()) }';
const binary = resolve(process.env.ZELYRA_BIN || "target/debug/zelyra");
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
let processHandle;
let browser;
let startupTimer;

try {
  await mkdir(join(project, "src"));
  await writeFile(join(project, "main.zyl"), main);
  await writeFile(join(project, "src/math.zyl"), "pub fn answer() -> Int requires { 1 < 2 } { return 42 }\npub fn dynamic(value: Int) requires { value >= 0 } {}\n");
  const port = await new Promise((resolvePort, reject) => {
    const socket = createServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const chosen = socket.address().port;
      socket.close(() => resolvePort(chosen));
    });
  });
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  const url = await Promise.race([
    new Promise((resolveUrl, reject) => {
      let output = "";
      processHandle.stdout.on("data", (chunk) => {
        output += chunk.toString();
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/);
        if (match) resolveUrl(match[0]);
      });
      processHandle.once("exit", (code) => reject(new Error(`Studio exited before startup: ${code}`)));
    }),
    new Promise((_, reject) => { startupTimer = setTimeout(() => reject(new Error("Studio startup timed out")), 10_000); }),
  ]);
  clearTimeout(startupTimer);
  const token = url.split("#")[1];
  browser = await engine.launch({ headless: true });
  const page = await browser.newPage();
  const errors = [];
  const semanticCalls = [];
  const checkSources = [];
  const holeCalls = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("response", async (response) => {
    if (response.url().endsWith("/api/semantic")) semanticCalls.push({
      status: response.status(), origin: response.request().headers().origin,
      referer: response.request().headers().referer,
    });
    if (response.url().endsWith("/api/typed-hole")) holeCalls.push({status:response.status(),body:await response.json().catch(() => null)});
  });
  page.on("request", (request) => {
    if (request.url().endsWith("/api/check")) {
      try { checkSources.push(request.postDataJSON()?.source); } catch { /* diagnostic only */ }
    }
  });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  const direct = await page.evaluate(async ({ token, main }) => {
    const offset = main.indexOf("answer") + 1;
    const response = await fetch("/__zelyra/editor/api/semantic", {
      method: "POST",
      headers: { "X-Zelyra-Editor-Token": token, "Content-Type": "application/json" },
      body: JSON.stringify({ path: "main.zyl", source: main, offset, action: "definition" }),
    });
    return { status: response.status, body: await response.json() };
  }, { token, main });
  assert.equal(direct.status, 200);
  assert.equal(direct.body.path, "src/math.zyl");
  const target = await page.evaluate(() => {
    const walker = document.createTreeWalker(document.querySelector(".cm-content"), NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const offset = walker.currentNode.textContent.indexOf("answer");
      if (offset < 0) continue;
      const range = document.createRange();
      range.setStart(walker.currentNode, offset + 1);
      range.setEnd(walker.currentNode, offset + 2);
      const box = range.getBoundingClientRect();
      return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    }
    return null;
  });
  assert.ok(target, "answer token is visible in CodeMirror");
  await page.mouse.click(target.x, target.y);
  await page.keyboard.press("F12");
  try {
    await page.waitForFunction(() => document.querySelector("#active-file")?.textContent === "src/math.zyl", undefined, { timeout: 10_000 });
  await page.locator(".verify-gutter .verify-marker.proven").waitFor({ timeout: 10_000 });
  await page.locator(".verify-gutter .verify-marker.runtime_check").waitFor({ timeout: 10_000 });
  assert.match(await page.locator(".verify-gutter .verify-marker.proven").getAttribute("aria-label"), /PROVEN V-001/);
  const holeSource = "pub fn answer() -> Int requires { 1 < 2 } { return 42 }\n"
    + "pub fn pure() -> String { return \"x\" }\n"
    + "pub fn target(name: String) -> String { message: String = _ return message }\n";
  const answerPoint = await page.evaluate(() => {
    const walker = document.createTreeWalker(document.querySelector(".cm-content"), NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const offset = walker.currentNode.textContent.indexOf("answer");
      if (offset < 0) continue;
      const range = document.createRange();
      range.setStart(walker.currentNode, offset + 1);
      range.setEnd(walker.currentNode, offset + 2);
      const box = range.getBoundingClientRect();
      return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    }
    return null;
  });
  assert.ok(answerPoint);
  await page.mouse.click(answerPoint.x, answerPoint.y);
  await page.keyboard.press("Control+A");
  await page.keyboard.type(holeSource, { delay: 10 });
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("message: String = _"), undefined, { timeout: 3_000 });
  await page.locator(".verify-gutter .verify-marker.proven").waitFor({ state: "detached", timeout: 10_000 });
  await page.locator("#problems-panel:not([hidden])").waitFor({ timeout: 10_000 });
  await page.waitForFunction(() => document.querySelector("#problems-list")?.textContent?.includes("E-HOLE-001"), undefined, { timeout: 10_000 });
  const holePoint = await page.evaluate(() => {
    const walker = document.createTreeWalker(document.querySelector(".cm-content"), NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const offset = walker.currentNode.textContent.indexOf("_");
      if (offset < 0) continue;
      const range = document.createRange();
      range.setStart(walker.currentNode, offset);
      range.setEnd(walker.currentNode, offset + 1);
      const box = range.getBoundingClientRect();
      return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    }
    return null;
  });
  assert.ok(holePoint, "the expression hole is visible in CodeMirror");
  await page.mouse.click(holePoint.x, holePoint.y);
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Control+Space");
  const nameSuggestion = page.locator(".cm-tooltip-autocomplete li").filter({ hasText: /^name/ });
  await nameSuggestion.waitFor({ timeout: 10_000 });
  await nameSuggestion.click();
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("message: String = name"), undefined, { timeout: 10_000 });
  await page.keyboard.press("Control+Z");
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("message: String = _"), undefined, { timeout: 10_000 });
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText("pub fn answer( {\n");
  await page.locator("#problems-panel:not([hidden])").waitFor({ timeout: 10_000 });
  } catch (error) {
    const state = await page.evaluate(() => ({
      active: document.querySelector("#active-file")?.textContent,
      status: document.querySelector("#status-message")?.textContent,
      selected: document.getSelection()?.anchorNode?.textContent,
      offset: document.getSelection()?.anchorOffset,
      problems: document.querySelector("#problems-list")?.textContent,
      source: document.querySelector(".cm-content")?.textContent,
      activeElement: document.activeElement?.outerHTML?.slice(0, 220),
      autocomplete: document.querySelector(".cm-tooltip-autocomplete")?.textContent,
    }));
    throw new Error(`${error.stack}; state=${JSON.stringify(state)}; calls=${JSON.stringify(semanticCalls)}; holeCalls=${JSON.stringify(holeCalls)}; checks=${JSON.stringify(checkSources.slice(-3))}; browserErrors=${JSON.stringify(errors)}`);
  }
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio semantic navigation, verifier gutter, and typed holes passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
