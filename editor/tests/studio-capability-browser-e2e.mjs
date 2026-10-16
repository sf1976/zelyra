import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-capability-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const originalConfig = `name = "capability-browser-e2e"
# The grant must preserve this comment.
[capabilities]
filesystem = false

[filesystem]
write_roots = ["data"]
`;
let processHandle;
let browser;

try {
  await writeFile(join(project, "zelyra.toml"), originalConfig);
  await mkdir(join(project, "data"));
  await writeFile(join(project, "main.zyl"), `fn main() uses FileSystem {
    write_text("data/result.txt", "generated only when the program runs")
}
`);
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
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator("#capability-button").click();
  await page.waitForFunction(() => {
    const status = document.querySelector("#capability-status")?.textContent || "";
    return Boolean(document.querySelector("#capability-needed button")) || (status && !/wird geladen|loading/i.test(status));
  });
  assert.ok(
    await page.locator('#capability-needed button').count(),
    `no missing capability grant button; status=${await page.locator("#capability-status").textContent()}, needed=${await page.locator("#capability-needed").textContent()}`,
  );
  await page.locator('#capability-needed button').filter({ hasText: "FileSystem" }).waitFor();
  await page.locator('#capability-needed button').filter({ hasText: "FileSystem" }).click();
  await page.locator("#capability-preview:not([hidden])").waitFor();
  const proposed = await page.locator("#capability-preview-code").textContent();
  assert.match(proposed, /filesystem\s*=\s*true/);
  assert.match(proposed, /The grant must preserve this comment\./);

  const token = new URL(url).hash.slice(1);
  const readConfig = async () => page.evaluate(async (token) => {
    const response = await fetch("/__zelyra/editor/api/file?path=zelyra.toml", {
      headers: { "X-Zelyra-Editor-Token": token },
    });
    return (await response.json()).content;
  }, token);
  assert.equal(await readConfig(), originalConfig, "preview must not write the project configuration");
  await page.locator("#capability-grant-apply").click();
  await page.waitForFunction(() => document.querySelector("#capability-status")?.textContent?.includes("gespeichert"));
  const updatedConfig = await readConfig();
  assert.match(updatedConfig, /filesystem\s*=\s*true/);
  assert.match(updatedConfig, /The grant must preserve this comment\./);
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio Capability grant required explicit review and preserved TOML in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
