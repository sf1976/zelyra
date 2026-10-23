import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-next-step-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const source = "table customers { id: Id primary auto name: String? }\n\nfn main() {}\n";
let server;
let browser;
let startupTimer;
try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "next-step-browser"\n');
  await writeFile(join(project, "main.zyl"), source);
  const port = await new Promise((resolvePort, reject) => {
    const socket = createServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const selected = socket.address().port;
      socket.close(() => resolvePort(selected));
    });
  });
  server = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], { stdio: ["ignore", "pipe", "pipe"] });
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
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+Home");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("End");
  await page.keyboard.press("Control+Space");
  const completion = page.locator(".cm-tooltip-autocomplete");
  await completion.waitFor({ timeout: 10_000 });
  const labels = await completion.locator(".cm-completionLabel").allTextContents();
  assert.ok(labels.includes("crud Customers -> customers {}"), `CRUD for the declared table was not suggested: ${labels}`);
  await completion.locator(".cm-completionLabel", { hasText: "crud Customers -> customers {}" }).click();
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("crud Customers -> customers {}"));
  const currentLines = await page.locator(".cm-line").allTextContents();
  assert.deepEqual(currentLines.slice(0, 4), [
    "table customers { id: Id primary auto name: String? }",
    "crud Customers -> customers {}",
    "fn main() {}",
    "",
  ]);
  await page.locator("#check-button").click();
  await page.waitForFunction(() => document.querySelector("#problems-list")?.textContent?.includes("Keine Probleme")
    || document.querySelector("#status-message")?.textContent?.includes("erfolgreich"), undefined, { timeout: 10_000 });
  await page.locator(".cm-content").click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText("table customers { id: Id primary auto name: String? }\nfn main() {\n\n}\n");
  await page.keyboard.press("Control+Home");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Control+Space");
  const nestedCompletion = page.locator(".cm-tooltip-autocomplete");
  await nestedCompletion.waitFor({ timeout: 10_000 });
  const nestedLabels = await nestedCompletion.locator(".cm-completionLabel").allTextContents();
  assert.ok(!nestedLabels.includes("crud Customers -> customers {}"), "top-level CRUD blocks must not be suggested inside a function");
  assert.deepEqual(errors, []);
  console.log("Studio compiler-guided CRUD completion browser flow passed");
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (server && server.exitCode === null) server.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
