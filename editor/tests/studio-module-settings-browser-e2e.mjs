import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile, mkdir } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";
import { clickStudioMenuAction } from "./studio-menu-test-helpers.mjs";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-module-settings-"));
const binary = resolve(process.env.ZELYRA_BIN || "../target/debug/zelyra");
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
let processHandle;
let browser;
try {
  await mkdir(join(project, "packages/ledger/src"), { recursive: true });
  await writeFile(join(project, "zelyra.toml"), `[package]\nname = "settings_browser_app"\nversion = "1.0.0"\nentry = "main.zyl"\n\n[dependencies.ledger]\npath = "packages/ledger"\nversion = "^1.0"\n`);
  await writeFile(join(project, "main.zyl"), "fn main() {}\n");
  await writeFile(join(project, "packages/ledger/zelyra.toml"), `[package]\nname = "ledger"\nversion = "1.0.0"\nentry = "src/lib.zyl"\n\n[settings.timeout]\ntype = "Int"\nrequired = true\ndescription = "Maximum wait in seconds"\n\n[settings.rate]\ntype = "Float"\nrequired = true\n\n[settings.api_token]\ntype = "String"\nrequired = true\nsecret = true\n`);
  await writeFile(join(project, "packages/ledger/src/lib.zyl"), "pub fn timeout() -> Int { module_setting<Int>(\"timeout\") }\n");
  const port = await new Promise((resolvePort, reject) => {
    const server = createServer(); server.once("error", reject);
    server.listen(0, "127.0.0.1", () => { const selected = server.address().port; server.close(() => resolvePort(selected)); });
  });
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], { stdio: ["ignore", "pipe", "pipe"] });
  const url = await new Promise((resolveUrl, reject) => {
    let output = "";
    processHandle.stdout.on("data", (chunk) => { output += chunk.toString(); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/); if (match) resolveUrl(match[0]); });
    processHandle.once("error", reject);
    processHandle.once("exit", (code) => reject(new Error(`Studio exited before startup: ${code}`)));
    setTimeout(() => reject(new Error("Studio startup timed out")), 10_000);
  });
  browser = await engine.launch({ headless: true });
  const page = await browser.newPage();
  const errors = []; page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await clickStudioMenuAction(page, "project", "module-settings-button");
  await page.locator("#module-settings-dialog[open]").waitFor();
  const timeout = page.locator('input[data-setting-name="timeout"]');
  await timeout.waitFor();
  await timeout.fill("37");
  await page.locator('input[data-setting-name="rate"]').fill("2");
  await page.locator('input[data-setting-name="api_token"]').fill("LEDGER_TOKEN");
  await page.locator("#module-settings-save").click();
  await page.locator("#module-settings-status").getByText("gespeichert").waitFor();
  const token = new URL(url).hash.slice(1);
  const result = await page.evaluate(async (token) => {
    const response = await fetch("/__zelyra/editor/api/module-settings", { headers: { "X-Zelyra-Editor-Token": token } });
    return { status: response.status, text: await response.text() };
  }, token);
  assert.equal(result.status, 200);
  assert.equal(result.text.includes("LEDGER_TOKEN"), false, "stored environment variable name must not be returned");
  assert.equal(result.text.includes("secret-value"), false);
  const settingsFile = await page.evaluate(async (token) => {
    const response = await fetch("/__zelyra/editor/api/file?path=zelyra.settings.toml", { headers: { "X-Zelyra-Editor-Token": token } });
    return (await response.json()).content;
  }, token);
  assert.match(settingsFile, /timeout\s*=\s*37/);
  assert.match(settingsFile, /rate\s*=\s*2\.0/);
  assert.match(settingsFile, /api_token\s*=\s*\{ env = "LEDGER_TOKEN" \}/);
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio module settings save and secret redaction passed in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
