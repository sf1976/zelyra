import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-semantic-"));
const main = 'import "src/math.zyl" as math\nfn main() { print(math::answer()) }';
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
let startupTimer;

try {
  await mkdir(join(project, "src"));
  await writeFile(join(project, "main.zyl"), main);
  await writeFile(join(project, "src/math.zyl"), "pub fn answer() -> Int requires { 1 < 2 } { return 42 }\npub fn dynamic(value: Int) requires { value >= 0 } {}\nfn private_helper() {}\n");
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
      request: response.request().postDataJSON(), result: await response.json().catch(() => null),
    });
    if (response.url().endsWith("/api/typed-hole")) holeCalls.push({status:response.status(),body:await response.json().catch(() => null)});
  });
  page.on("request", (request) => {
    if (request.url().endsWith("/api/check")) {
      try { checkSources.push(request.postDataJSON()?.source); } catch { /* diagnostic only */ }
    }
  });
  const eventHandshake = page.waitForResponse((response) => response.url().endsWith("/api/events"), { timeout: 10_000 });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  assert.equal((await eventHandshake).status(), 200, "Studio opens its authenticated event stream");
  const eventsUrl = `${new URL(url).origin}/__zelyra/editor/api/events`;
  const deniedStream = await fetch(eventsUrl);
  assert.equal(deniedStream.status, 403, "event stream rejects missing session token");
  const eventAbort = new AbortController();
  try {
    const stream = await fetch(eventsUrl, {
      headers: { "X-Zelyra-Editor-Token": token, Origin: new URL(url).origin },
      signal: eventAbort.signal,
    });
    assert.equal(stream.status, 200);
    const checked = await fetch(`${new URL(url).origin}/__zelyra/editor/api/check`, {
      method: "POST",
      headers: { "X-Zelyra-Editor-Token": token, "Content-Type": "application/json", Origin: new URL(url).origin },
      body: JSON.stringify({ path: "main.zyl", source: main, version: 987, clientId: "test-client" }),
    });
    assert.equal(checked.status, 200);
    const reader = stream.body.getReader();
    let eventText = "";
    let diagnosticEvent = null;
    const deadline = Date.now() + 5_000;
    while (!diagnosticEvent && Date.now() < deadline) {
      const { value, done } = await Promise.race([
        reader.read(),
        new Promise((_, reject) => setTimeout(() => reject(new Error("diagnostic SSE event timed out")), 5_000)),
      ]);
      if (done) break;
      eventText += new TextDecoder().decode(value);
      const frames = eventText.split("\n\n");
      eventText = frames.pop();
      for (const frame of frames) {
        const data = frame.split("\n").find((line) => line.startsWith("data: "));
        if (!data) continue;
        const event = JSON.parse(data.slice(6));
        if (event.kind === "diagnostics") diagnosticEvent = event;
      }
    }
    assert.equal(diagnosticEvent?.schema, 1);
    assert.equal(diagnosticEvent?.document.path, "main.zyl");
    assert.equal(diagnosticEvent?.document.version, 987);
    assert.equal(diagnosticEvent?.document.clientId, "test-client");
  } finally {
    eventAbort.abort();
  }
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator(".cm-content[contenteditable=true]").click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('import "src/ma');
  const importPathCompletion = page.locator(".cm-tooltip-autocomplete");
  await importPathCompletion.waitFor();
  const importPathLabels = await importPathCompletion.locator(".cm-completionLabel").allTextContents();
  assert.ok(importPathLabels.includes("src/math.zyl"), `project import path missing: ${importPathLabels}`);
  assert.ok(!importPathLabels.includes("main.zyl"), `entry point should not be suggested as a dependency: ${importPathLabels}`);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('import "src/math.zyl" as math\nfn main() { let value = math::');
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+Space");
  const moduleCompletion = page.locator(".cm-tooltip-autocomplete");
  await moduleCompletion.waitFor();
  const memberLabels = await moduleCompletion.locator(".cm-completionLabel").allTextContents();
  assert.ok(memberLabels.includes("answer"), `imported public member missing: ${memberLabels}`);
  assert.ok(memberLabels.includes("dynamic"), `second imported public member missing: ${memberLabels}`);
  assert.ok(!memberLabels.includes("private_helper"), `private module member leaked: ${memberLabels}`);
  assert.ok(!memberLabels.includes("main"), `global suggestions leaked into namespace completion: ${memberLabels}`);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText(main);
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
  await page.locator('.tree-item.file[title="src/math.zyl"]').click();
  await page.locator('.cm-content[contenteditable=true]').click();
  await page.keyboard.press("Control+Home");
  await page.keyboard.insertText("// unsaved module line\n");
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("unsaved module line"));
  await page.locator('#tabs .tab').filter({ hasText: 'main.zyl' }).click();
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
  assert.match(await page.locator(".cm-content").textContent(), /unsaved module line/, "F12 preserves the unsaved target buffer");
  assert.ok(semanticCalls.some((call) => call.status === 200
    && call.request.overlays?.some((overlay) => overlay.path === "src/math.zyl"
      && overlay.source.startsWith("// unsaved module line\n"))
    && call.result.start > direct.body.start), "F12 resolves the second unsaved module's shifted source position");
  await page.locator(".verify-gutter .verify-marker.proven").waitFor({ timeout: 10_000 });
  await page.locator(".verify-gutter .verify-marker.runtime_check").waitFor({ timeout: 10_000 });
  assert.match(await page.locator(".verify-gutter .verify-marker.proven").getAttribute("aria-label"), /PROVEN V-001/);
  const holeSource = "pub fn answer() -> Int requires { 1 < 2 } { return 42 }\n"
    + "pub fn pure() -> String { return \"x\" }\n"
    + "pub fn target(name: String) -> String {\n inferred_name = pure()\n message: String = _\n return message\n";
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
      const line = walker.currentNode.parentElement?.closest(".cm-line")?.textContent || "";
      if (!line.includes("= _")) continue;
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
  const inferredSuggestion = page.locator(".cm-tooltip-autocomplete li").filter({ hasText: /^inferred_name/ });
  await inferredSuggestion.waitFor({ timeout: 10_000 });
  await inferredSuggestion.click();
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("message: String = inferred_name"), undefined, { timeout: 10_000 });
  await page.keyboard.press("Control+Z");
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("message: String = _"), undefined, { timeout: 10_000 });
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
