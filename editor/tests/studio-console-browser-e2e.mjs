import assert from "node:assert/strict";
import { spawnSync, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-console-browser-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : resolve("target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const source = `fn main() uses Console {
    name = read_console("Name? ")
    match name {
        Some(value) => { print("Hello " + value) }
        None => { print("EOF") }
    }
}
`;
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"),
    '[project]\nname = "studio-console-e2e"\n\n[capabilities]\nconsole = true\n');
  await writeFile(join(project, "main.zyl"), source);

  const cliRun = spawnSync(binary, ["run", "main.zyl"], {
    cwd: project, input: "Ada\n", encoding: "utf8", timeout: 10_000,
  });
  assert.equal(cliRun.status, 0, cliRun.stderr);
  assert.match(cliRun.stdout, /Name\? Hello Ada/);

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
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 800, height: 800 } });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const logoResponse = page.waitForResponse((response) =>
    response.url().endsWith("/__zelyra/editor/assets/zelyra-logo.png"));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  const logo = await logoResponse;
  assert.equal(logo.status(), 200);
  assert.match(logo.headers()["content-type"], /image\/png/);
  await page.waitForFunction(() => document.querySelector(".studio-splash-logo img")?.naturalWidth > 0);
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();

  await page.locator("#run-button").click();
  await page.locator("#run-console-form:not([hidden])").waitFor();
  await page.waitForFunction(() => document.querySelector("#run-console-input")?.disabled === false);
  await page.waitForFunction(() => document.querySelector("#run-output-content")?.textContent?.includes("Name? "));
  await page.locator("#run-console-input").fill("Ada");
  await page.locator("#run-console-submit").click();
  await page.waitForFunction(() => /erfolgreich beendet|finished successfully/i.test(
    document.querySelector("#run-output-state")?.textContent || ""));
  const studioOutput = await page.locator("#run-output-content").textContent();
  assert.match(studioOutput, /Hello Ada/);
  assert.ok(studioOutput.includes(cliRun.stdout.trim()),
    `Studio output should match CLI output. CLI=${JSON.stringify(cliRun.stdout)} Studio=${JSON.stringify(studioOutput)}`);

  await page.locator("#run-output-close").click();
  await page.locator("#run-button").click();
  await page.locator("#run-console-form:not([hidden])").waitFor();
  await page.waitForFunction(() => document.querySelector("#run-console-input")?.disabled === false);
  await page.locator("#run-console-eof").click();
  await page.waitForFunction(() => /erfolgreich beendet|finished successfully/i.test(
    document.querySelector("#run-output-state")?.textContent || ""));
  assert.match(await page.locator("#run-output-content").textContent(), /EOF/);
  assert.deepEqual(pageErrors, []);
  console.log("Studio interactive console matches CLI input and supports explicit EOF");
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
