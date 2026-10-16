import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const project = await mkdtemp(join(tmpdir(), "zelyra-pipeline-browser-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const source = "database main { engine: mariadb }\n"
  + "table items { id: Id primary auto name: String? }\n"
  + "struct ItemRow { id: Id name: String? }\n"
  + "tableview Items { source sql<ItemRow[]> { SELECT id, name FROM items } columns { id name } }\n";
const changed = source.replace("name: String? }\ntableview", "name: String }\ntableview");
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "pipeline-browser"\n');
  await writeFile(join(project, "main.zyl"), source);
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
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+End");
  await page.waitForFunction(() => document.querySelector("#pipeline-breadcrumbs")?.textContent?.includes("sql:"),
    undefined, { timeout: 10_000 });
  assert.match(await page.locator("#pipeline-breadcrumbs").textContent(), /table:/);
  await page.locator("#project-map-button").click();
  await page.locator(".project-map-node.kind-sql").first().waitFor();
  await page.locator(".project-map-node.kind-struct").first().waitFor();
  await page.locator("#project-map-close").click();
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+A");
  await page.keyboard.type(changed, { delay: 1 });
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("struct ItemRow { id: Id name: String }"),
    undefined, { timeout: 10_000 });
  await page.waitForFunction(() => document.querySelector("#problems-list")?.textContent?.includes("E-SQL-004"),
    undefined, { timeout: 10_000 });
  await page.locator("#project-map-button").click();
  await page.locator("#project-map-nullability:not([hidden])").waitFor({ timeout: 10_000 });
  assert.match(await page.locator("#project-map-nullability-list").textContent(), /E-SQL-004/);
  assert.equal(await readFile(join(project, "main.zyl"), "utf8"), source,
    "the project map must analyze the unsaved buffer without changing disk files");
  assert.deepEqual(errors, []);
  console.log(`Studio pipeline graph and nullability browser flow passed in ${engineName}`);
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
