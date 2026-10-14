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
const project = await mkdtemp(join(tmpdir(), "zelyra-outbox-browser-"));
const binary = resolve(process.env.ZELYRA_BIN || "target/debug/zelyra");
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "outbox-browser"\n');
  await writeFile(join(project, "main.zyl"), "database main { engine: mariadb } fn main() {}\n");
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
  const page = await browser.newPage({ viewport: { width: 360, height: 760 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator("#outbox-button").click();
  await page.waitForFunction(() => document.querySelector("#outbox-status")?.textContent?.includes("MariaDB"));
  assert.match(await page.locator("#outbox-status").textContent(), /DATABASE_URL/);
  assert.equal(await page.locator("#outbox-rows tr").count(), 0);
  assert.equal(await page.locator("#outbox-next").isDisabled(), true);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(overflow <= 1, `page overflowed horizontally by ${overflow}px`);
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#outbox-dialog").evaluate((dialog) => dialog.open), false);
  assert.deepEqual(errors, []);
  console.log(`Studio outbox missing-config browser state passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
