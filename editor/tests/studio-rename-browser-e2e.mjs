import assert from "node:assert/strict";
import axe from "axe-core";
import pixelmatch from "pixelmatch";
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";
import { PNG } from "pngjs";

async function assertStudioScreenshot(page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    document.querySelector("#project-name").textContent = "Studio Test";
    document.querySelector("#project-name-side").textContent = "STUDIO TEST";
  });
  const actualBuffer = await page.screenshot({ fullPage: false, animations: "disabled" });
  const baselinePath = new URL("./visual-baselines/studio-main-dark-desktop.png", import.meta.url);
  if (process.env.ZELYRA_UPDATE_VISUAL_BASELINES === "1") {
    await mkdir(new URL("./visual-baselines/", import.meta.url), { recursive: true });
    await writeFile(baselinePath, actualBuffer);
    return;
  }
  const baseline = PNG.sync.read(await readFile(baselinePath));
  const actual = PNG.sync.read(actualBuffer);
  assert.deepEqual({ width: actual.width, height: actual.height }, { width: baseline.width, height: baseline.height }, "Studio screenshot dimensions changed");
  const diff = new PNG({ width: actual.width, height: actual.height });
  const changed = pixelmatch(baseline.data, actual.data, diff.data, actual.width, actual.height, { threshold: 0.2 });
  const limit = Math.ceil(actual.width * actual.height * 0.002);
  assert.ok(changed <= limit, `Studio visual regression: ${changed} changed pixels, limit ${limit}`);
}

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-rename-"));
const main = 'import "src/math.zyl" as math\nfn retain(record: math::Answer) -> math::Answer { return record }\nfn main() { print(math::answer()) customer: math::Answer = math::Answer { value: 42 } print(customer.value) }\n';
const math = "pub fn answer() -> Int { return 42 }\npub struct Answer { value: Int }\n";
const binary = resolve(process.env.ZELYRA_BIN || "../target/debug/zelyra");
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
let processHandle;
let browser;

try {
  await mkdir(join(project, "src"));
  await writeFile(join(project, "zelyra.toml"), 'name = "studio-browser-e2e"\n');
  await writeFile(join(project, "main.zyl"), main);
  await writeFile(join(project, "src/math.zyl"), math);
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
  let firstUsableMs = null;
  let analysisMs = null;
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const navigationStartedAt = performance.now();
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  firstUsableMs = performance.now() - navigationStartedAt;
  await page.setViewportSize({ width: 1280, height: 900 });
  if (engineName === "chromium") await assertStudioScreenshot(page);
  const checkResponse = page.waitForResponse((response) => response.url().endsWith("/api/check"));
  const analysisStartedAt = performance.now();
  await page.locator("#check-button").click();
  await checkResponse;
  analysisMs = performance.now() - analysisStartedAt;
  await page.locator('.tree-item.file[title="src/math.zyl"]').click();
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+Home");
  await page.keyboard.insertText("// unsaved target\n");
  await page.locator('#tabs .tab').filter({ hasText: "main.zyl" }).click();
  const token = url.split("#")[1];
  const point = await page.evaluate(() => {
    const walker = document.createTreeWalker(document.querySelector(".cm-content"), NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const offset = walker.currentNode.textContent.indexOf("answer");
      if (offset < 0) continue;
      const range = document.createRange();
      range.setStart(walker.currentNode, offset + 1);
      range.setEnd(walker.currentNode, offset + 2);
      const rect = range.getBoundingClientRect();
      return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    }
    return null;
  });
  assert.ok(point, "the imported function call is visible");
  await page.mouse.click(point.x, point.y);
  await page.keyboard.press("F2");
  await page.locator("#semantic-rename-dialog[open]").waitFor();
  await page.locator("#semantic-rename-name").fill("calculate_answer");
  await page.locator("#semantic-rename-preview-button").click();
  await page.waitForFunction(() => document.querySelector("#semantic-rename-status")?.textContent?.includes("2 Dateien"));
  await page.locator("#semantic-rename-apply-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("2 Dateien geändert"));
  assert.match(await page.locator(".cm-content").textContent(), /math::calculate_answer\(\)/);
  await page.locator('#tabs .tab').filter({ hasText: "math.zyl" }).click();
  const targetSource = await page.locator(".cm-content").textContent();
  assert.match(targetSource, /unsaved target/);
  assert.match(targetSource, /fn calculate_answer\(\)/);
  const saved = await page.evaluate(async ({ token }) => {
    const response = await fetch("/__zelyra/editor/api/file?path=src%2Fmath.zyl", {
      headers: { "X-Zelyra-Editor-Token": token },
    });
    return response.json();
  }, { token });
  assert.equal(saved.content, math, "semantic refactoring leaves disk unchanged until the user saves");
  await page.locator("#save-all-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("2 Dateien gemeinsam gespeichert"));
  const savedFiles = await page.evaluate(async ({ token }) => {
    const read = async (path) => (await fetch(`/__zelyra/editor/api/file?path=${encodeURIComponent(path)}`, {
      headers: { "X-Zelyra-Editor-Token": token },
    })).json();
    return Promise.all([read("main.zyl"), read("src/math.zyl")]);
  }, { token });
  assert.match(savedFiles[0].content, /math::calculate_answer\(\)/);
  assert.match(savedFiles[1].content, /fn calculate_answer\(\)/);
  await page.locator('#tabs .tab').filter({ hasText: "main.zyl" }).click();
  const recordPoint = await page.evaluate(() => {
    const walker = document.createTreeWalker(document.querySelector(".cm-content"), NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const offset = walker.currentNode.textContent.indexOf("Answer");
      if (offset < 0) continue;
      const range = document.createRange();
      range.setStart(walker.currentNode, offset + 1);
      range.setEnd(walker.currentNode, offset + 2);
      const rect = range.getBoundingClientRect();
      return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    }
    return null;
  });
  assert.ok(recordPoint, "the imported record constructor is visible");
  await page.mouse.click(recordPoint.x, recordPoint.y);
  await page.keyboard.press("F2");
  await page.locator("#semantic-rename-dialog[open]").waitFor();
  await page.locator("#semantic-rename-name").fill("Invoice");
  await page.locator("#semantic-rename-preview-button").click();
  await page.waitForFunction(() => document.querySelector("#semantic-rename-status")?.textContent?.includes("2 Dateien"));
  await page.locator("#semantic-rename-apply-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("2 Dateien geändert"));
  assert.match(await page.locator(".cm-content").textContent(), /math::Invoice \{ value: 42 \}/);
  assert.match(await page.locator(".cm-content").textContent(), /fn retain\(record: math::Invoice\) -> math::Invoice/);
  assert.match(await page.locator(".cm-content").textContent(), /customer: math::Invoice = math::Invoice/);
  await page.locator('#tabs .tab').filter({ hasText: "math.zyl" }).click();
  assert.match(await page.locator(".cm-content").textContent(), /struct Invoice \{ value: Int \}/);
  await page.locator("#save-all-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("2 Dateien gemeinsam gespeichert"));
  await page.locator('#tabs .tab').filter({ hasText: "main.zyl" }).click();
  const fieldPoint = await page.evaluate(() => {
    const walker = document.createTreeWalker(document.querySelector(".cm-content"), NodeFilter.SHOW_TEXT);
    let point = null;
    while (walker.nextNode()) {
      if (walker.currentNode.textContent !== "value") continue;
      const range = document.createRange();
      range.selectNodeContents(walker.currentNode);
      const rect = range.getBoundingClientRect();
      point = { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    }
    return point;
  });
  assert.ok(fieldPoint, "the typed record field access is visible");
  await page.mouse.click(fieldPoint.x, fieldPoint.y);
  await page.keyboard.press("F2");
  await page.locator("#semantic-rename-dialog[open]").waitFor();
  await page.locator("#semantic-rename-name").fill("amount");
  await page.locator("#semantic-rename-preview-button").click();
  await page.waitForFunction(() => document.querySelector("#semantic-rename-status")?.textContent?.includes("2 Dateien"));
  await page.locator("#semantic-rename-apply-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("2 Dateien geändert"));
  assert.match(await page.locator(".cm-content").textContent(), /customer: math::Invoice = math::Invoice \{ amount: 42 \} print\(customer.amount\)/);
  await page.locator('#tabs .tab').filter({ hasText: "math.zyl" }).click();
  assert.match(await page.locator(".cm-content").textContent(), /struct Invoice \{ amount: Int \}/);
  await page.locator("#save-all-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("2 Dateien gemeinsam gespeichert"));
  page.on("dialog", async (dialog) => {
    if (dialog.type() === "prompt") await dialog.accept("src/answers.zyl");
    else await dialog.accept();
  });
  await page.locator('#tabs .tab').filter({ hasText: "math.zyl" }).click();
  await page.locator("#rename-file-button").click();
  await page.waitForFunction(() => document.querySelector("#status-message")?.textContent?.includes("Datei umbenannt") || document.querySelector("#status-message")?.dataset.kind === "error");
  assert.match(await page.locator("#status-message").textContent(), /Datei umbenannt/, `rename failed: ${await page.locator("#status-message").textContent()}, browser errors: ${errors.join(", ")}`);
  const afterRename = await page.evaluate(async ({ token }) => {
    const read = async (path) => (await fetch(`/__zelyra/editor/api/file?path=${encodeURIComponent(path)}`, {
      headers: { "X-Zelyra-Editor-Token": token },
    }));
    const old = await read("src/math.zyl");
    const next = await read("src/answers.zyl");
    const main = await read("main.zyl");
    return { oldStatus: old.status, next: await next.json(), main: await main.json() };
  }, { token });
  assert.equal(afterRename.oldStatus, 404, "old path is removed");
  assert.match(afterRename.next.content, /fn calculate_answer\(\)/);
  assert.match(afterRename.main.content, /import \"src\/answers.zyl\" as math/);
  await page.locator('#tabs .tab').filter({ hasText: "main.zyl" }).click();
  assert.match(await page.locator(".cm-content").textContent(), /import \"src\/answers.zyl\" as math/);
  for (const width of [360, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    await page.waitForFunction((expected) => window.innerWidth === expected, width);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), `Studio overflowed horizontally at ${width}px`);
  }
  // A 640x450 CSS viewport matches the layout area of a 1280x900 desktop at
  // 200% browser zoom. Keep this explicit so the zoom acceptance has coverage.
  await page.setViewportSize({ width: 640, height: 450 });
  assert.ok(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    "Studio must not overflow horizontally at the 640x450 CSS viewport equivalent to 200% zoom",
  );
  const sameOrigin = new URL(url).origin;
  await page.route(`${sameOrigin}/__zelyra/axe-test.js`, (route) => route.fulfill({
    status: 200, contentType: "text/javascript", body: axe.source,
  }));
  await page.addScriptTag({ url: `${sameOrigin}/__zelyra/axe-test.js` });
  for (const width of [360, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    for (let themeIndex = 0; themeIndex < 2; themeIndex += 1) {
      const accessibility = await page.evaluate(async () => axe.run(document, {
        runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
      }));
      const violations = accessibility.violations.map(({ id, impact, nodes }) => ({
        id, impact, nodes: nodes.map((node) => ({ target: node.target, html: node.html, summary: node.failureSummary })),
      }));
      assert.deepEqual(violations, [], `Studio shell must pass automated WCAG A/AA checks at ${width}px for theme ${themeIndex}`);
      await page.locator("#theme-button").click();
    }
  }
  await page.setViewportSize({ width: 640, height: 450 });
  for (let themeIndex = 0; themeIndex < 2; themeIndex += 1) {
    assert.ok(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      `Studio must not overflow at the 200% zoom-equivalent viewport for theme ${themeIndex}`,
    );
    const accessibility = await page.evaluate(async () => axe.run(document, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
    }));
    const violations = accessibility.violations.map(({ id, impact, nodes }) => ({
      id, impact, nodes: nodes.map((node) => ({ target: node.target, html: node.html, summary: node.failureSummary })),
    }));
    assert.deepEqual(violations, [], `Studio shell must pass automated WCAG A/AA checks at the 200% zoom-equivalent viewport for theme ${themeIndex}`);
    await page.locator("#theme-button").click();
  }
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio function, record, and field rename; journaled save; and import-aware file rename passed in ${engineName}; first usable editor ${firstUsableMs.toFixed(1)} ms; analysis response ${analysisMs?.toFixed(1) ?? "unmeasured"} ms`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
