import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine);
const project = await mkdtemp(join(tmpdir(), "zelyra-canvas-browser-"));
const binary = resolve(process.env.ZELYRA_BIN || "target/debug/zelyra");
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "canvas-browser"\n');
  await writeFile(join(project, "zelyra.theme.css"), '/* --z-color-primary: #000000; */\n:where(html[data-z-ui]) { --z-color-primary: #23654f; }\n');
  await writeFile(join(project, "main.zyl"),
    'page "/" { html { <main class="z-container"><h1>Welcome</h1><script>document.body.dataset.scriptRan="yes"</script></main> } }\nfn main() { assert(false) }\n');
  const port = await new Promise((done, reject) => {
    const socket = createServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const selected = socket.address().port;
      socket.close(() => done(selected));
    });
  });
  server = spawn(binary, ["editor", project, "--no-open", "--port", String(port)],
    { stdio: ["ignore", "pipe", "pipe"] });
  const url = await Promise.race([
    new Promise((done, reject) => {
      let output = "";
      server.stdout.on("data", (chunk) => {
        output += chunk.toString();
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/);
        if (match) done(match[0]);
      });
      server.once("exit", (code) => reject(new Error(`Studio exited: ${code}`)));
    }),
    new Promise((_, reject) => { startupTimer = setTimeout(() => reject(new Error("Studio startup timed out")), 10_000); }),
  ]);
  clearTimeout(startupTimer);
  browser = await engine.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 360, height: 760 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator("#canvas-button").click();
  await page.locator("#canvas-pane:not([hidden])").waitFor();
  const frame = page.frameLocator("#canvas-frame");
  await frame.locator("h1").waitFor();
  assert.equal(await frame.locator("h1").textContent(), "Welcome");
  assert.equal(await frame.locator("body").getAttribute("data-script-ran"), null);
  assert.equal(await frame.locator("html").getAttribute("data-z-ui"), "");
  assert.equal(await frame.locator("html").evaluate((node) => getComputedStyle(node).getPropertyValue("--z-color-primary").trim()), "#23654f");
  assert.equal(await page.locator("#canvas-target option").count(), 1);
  assert.equal(await page.locator("#canvas-source").isEnabled(), true);
  await page.waitForFunction(() => document.querySelector("#canvas-theme-token option")?.textContent?.includes("--z-color-primary"));
  assert.equal(await page.locator("#canvas-theme-token option").count(), 1);
  await page.locator("#canvas-theme-color").fill("#f0f000");
  await page.locator("#canvas-theme-apply").click();
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("#f0f000"));
  assert.match(await page.locator(".cm-content").textContent(), /--z-color-on-primary: #000000/);
  assert.match(await readFile(join(project, "zelyra.theme.css"), "utf8"), /#23654f/);
  await page.waitForFunction(() => document.querySelector("#canvas-status")?.textContent?.includes("Statisch gerendert")
    || document.querySelector("#canvas-status")?.textContent?.includes("Rendered statically"));
  assert.equal(await page.locator("#canvas-frame").getAttribute("sandbox"), "");
  let previewColor = "";
  for (let attempt = 0; attempt < 30; attempt++) {
    previewColor = await frame.locator("html").evaluate((node) =>
      getComputedStyle(node).getPropertyValue("--z-color-primary").trim());
    if (previewColor === "#f0f000") break;
    await page.waitForTimeout(100);
  }
  assert.equal(previewColor, "#f0f000", "the unsaved theme change must reach the isolated canvas");
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(overflow <= 1, `Studio overflowed by ${overflow}px at 360px`);
  await page.locator("#canvas-close").click();
  assert.equal(await page.locator("#canvas-pane").isHidden(), true);
  assert.deepEqual(errors, []);
  console.log(`Studio static canvas passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
