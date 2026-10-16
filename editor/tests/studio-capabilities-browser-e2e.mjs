import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const project = await mkdtemp(join(tmpdir(), "zelyra-capability-browser-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
let server;
let browser;
let startupTimer;
try {
  await mkdir(join(project, "notes"));
  await writeFile(join(project, "zelyra.toml"),
    '[project]\nname = "capability-browser"\n# preserve this comment\n[capabilities]\nfilesystem = false\n[filesystem]\nread_roots = ["."]\nwrite_roots = ["notes"]\n');
  await writeFile(join(project, "main.zyl"),
    'fn main() uses FileSystem { write_text("notes/a.txt", "ok") }\n');
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
  const page = await browser.newPage({ viewport: { width: 360, height: 760 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator("#capability-button").click();
  await page.locator("#capability-dialog[open]").waitFor();
  await page.waitForFunction(() => document.querySelector("#capability-needed")?.textContent?.includes("FileSystem"));
  assert.match(await page.locator("#capability-paths").textContent(), /notes\/a\.txt.*allowed/);
  assert.match(await page.locator("#capability-needed").textContent(), /nicht freigegeben|not granted/);
  await page.locator("#capability-needed button").click();
  await page.waitForFunction(() => document.querySelector("#capability-preview-code")?.textContent?.includes("filesystem = true"));
  assert.match(await page.locator("#capability-preview-code").textContent(), /filesystem = true/);
  assert.match(await readFile(join(project, "zelyra.toml"), "utf8"), /filesystem = false/);
  await page.locator("#capability-grant-apply").click();
  await page.waitForFunction(() => document.querySelector("#capability-status")?.textContent?.includes("Freigabe gespeichert")
    || document.querySelector("#capability-status")?.textContent?.includes("Grant saved"));
  const config = await readFile(join(project, "zelyra.toml"), "utf8");
  assert.match(config, /filesystem = true/);
  assert.match(config, /# preserve this comment/);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(overflow <= 1, `page overflowed horizontally by ${overflow}px`);
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#capability-dialog").evaluate((dialog) => dialog.open), false);
  assert.deepEqual(errors, []);
  console.log(`Studio capability matrix and reviewed grant passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
