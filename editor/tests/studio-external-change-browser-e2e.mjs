import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine);
const project = await mkdtemp(join(tmpdir(), "zelyra-external-browser-"));
const binary = resolve(process.env.ZELYRA_BIN || "target/debug/zelyra");
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "external-browser"\n');
  await writeFile(join(project, "main.zyl"), 'fn main() { print("initial") }\n');
  const port = await new Promise((done, reject) => {
    const socket = createServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const chosen = socket.address().port;
      socket.close(() => done(chosen));
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
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  const content = page.locator(".cm-content[contenteditable=true]");
  await content.waitFor();
  assert.match(await content.textContent(), /initial/);
  await page.waitForTimeout(1800); // allow the background watcher to establish its baseline
  await writeFile(join(project, "main.zyl"), 'fn main() { print("external") }\n');
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("external"), { timeout: 10_000 });
  assert.match(await page.locator("#status-message").textContent(), /Extern geänderte Datei|Reloaded externally changed file/);
  await content.click();
  await page.keyboard.press("ControlOrMeta+End");
  await page.keyboard.insertText("\n# local edit");
  assert.match(await content.textContent(), /local edit/);
  await writeFile(join(project, "main.zyl"), 'fn main() { print("second") }\n');
  await page.waitForFunction(() => /außerhalb von Studio geändert|changed outside Studio/.test(document.querySelector("#status-message")?.textContent || ""), { timeout: 10_000 });
  assert.match(await content.textContent(), /local edit/);
  assert.doesNotMatch(await content.textContent(), /second/);
  assert.deepEqual(errors, []);
  console.log(`Studio external workspace changes passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
