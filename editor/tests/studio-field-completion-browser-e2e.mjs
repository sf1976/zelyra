import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-fields-"));
const source = `struct Customer { name: String, email: String }
fn main() {
    customer: Customer = Customer { name: "Ada", email: "ada@example.test" }
    print(customer.na)
}
`;
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : resolve("target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
let processHandle;
let browser;
let startupTimer;

try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "studio-field-completion-e2e"\n');
  await writeFile(join(project, "main.zyl"), source);
  const port = await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const selected = server.address().port;
      server.close(() => resolvePort(selected));
    });
  });
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  const url = await Promise.race([
    new Promise((resolveUrl, reject) => {
      let output = "";
      processHandle.stdout.on("data", (chunk) => {
        output += chunk.toString();
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/__zelyra\/editor\/#([a-f0-9]{64})/);
        if (match) resolveUrl(match[0]);
      });
      processHandle.once("exit", (code) => reject(new Error(`Studio exited before startup: ${code}`)));
    }),
    new Promise((_, reject) => { startupTimer = setTimeout(() => reject(new Error("Studio startup timed out")), 10_000); }),
  ]);
  clearTimeout(startupTimer);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const errors = [];
  let completionRequest;
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (request.url().endsWith("/api/field-completion")) completionRequest = request.postDataJSON();
  });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  const editor = page.locator(".cm-content[contenteditable=true]");
  await editor.waitFor();
  const caret = await page.evaluate(() => {
    const root = document.querySelector(".cm-content");
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const target = "customer.na";
    const nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    const joined = nodes.map((node) => node.textContent).join("");
    const index = joined.indexOf(target);
    if (index >= 0) {
      const cursor = index + target.length;
      let consumed = 0;
      for (const node of nodes) {
        const end = consumed + node.textContent.length;
        if (cursor <= end) {
          const localOffset = cursor - consumed;
          if (localOffset > node.textContent.length) return null;
          const range = document.createRange();
          range.setStart(node, localOffset);
          range.collapse(true);
          const rect = range.getBoundingClientRect();
          return { x: rect.left, y: rect.top + rect.height / 2 };
        }
        consumed = end;
      }
    }
    return null;
  });
  assert.ok(caret, "field access is visible in the editor");
  await page.mouse.click(caret.x, caret.y);
  await page.keyboard.type("m");
  await page.locator(".cm-tooltip-autocomplete").waitFor();
  await page.locator(".cm-tooltip-autocomplete li").filter({ hasText: "name" }).waitFor();
  assert.ok(completionRequest, "Studio requests compiler-backed field completion");
  assert.equal(completionRequest.path, "main.zyl");
  assert.equal(completionRequest.source, source.replace("customer.na", "customer.nam"));
  assert.equal(completionRequest.dotEnd > 0, true);
  assert.equal(completionRequest.cursor > completionRequest.dotEnd, true);
  const options = await page.locator(".cm-tooltip-autocomplete li").allTextContents();
  assert.ok(options.some((option) => option.includes("name")));
  assert.ok(!options.some((option) => option.includes("email")), "typed prefix filters unrelated record fields");
  await page.locator(".cm-tooltip-autocomplete li").filter({ hasText: "name" }).click();
  await page.waitForFunction(() => document.querySelector(".cm-content")?.textContent?.includes("customer.name)"));
  assert.deepEqual(errors, []);
  console.log("Studio offers only compiler-resolved record fields and applies the selected completion");
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) processHandle.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
