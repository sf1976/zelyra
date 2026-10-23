import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-signature-"));
const source = `import "src/customer.zyl" as customers
fn main() { customers::find_customer(7, ) }
`;
const moduleSource = `pub fn find_customer(id: Int, include_archived: Bool) -> String { return "" }
`;
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : resolve("target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
let processHandle;
let browser;
let startupTimer;

try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "studio-signature-help-e2e"\n');
  await writeFile(join(project, "main.zyl"), source);
  await mkdir(join(project, "src"), { recursive: true });
  await writeFile(join(project, "src", "customer.zyl"), moduleSource);
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
  let signatureRequest;
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (request.url().endsWith("/api/signature-help")) signatureRequest = request.postDataJSON();
  });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  const editor = page.locator(".cm-content[contenteditable=true]");
  await editor.waitFor();
  const caret = await page.evaluate(() => {
    const root = document.querySelector(".cm-content");
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    const joined = nodes.map((node) => node.textContent).join("");
    const marker = "customers::find_customer(7, )";
    const index = joined.indexOf(marker);
    if (index < 0) return null;
    const cursor = index + marker.indexOf(", ") + 2;
    let consumed = 0;
    for (const node of nodes) {
      const end = consumed + node.textContent.length;
      if (cursor <= end) {
        const range = document.createRange();
        range.setStart(node, cursor - consumed);
        range.collapse(true);
        const rect = range.getBoundingClientRect();
        return { x: rect.left, y: rect.top + rect.height / 2 };
      }
      consumed = end;
    }
    return null;
  });
  assert.ok(caret, "cursor location inside the imported function call is visible");
  await page.mouse.click(caret.x, caret.y);
  const signature = page.locator("#signature-help:not([hidden])");
  await signature.waitFor();
  await page.getByText("find_customer", { exact: false }).first().waitFor();
  assert.match(await signature.textContent(), /id: Int, include_archived: Bool/);
  assert.match(await signature.textContent(), /Parameter 2 of 2|Parameter 2 von 2/);
  assert.ok(signatureRequest, "Studio asks the compiler for the active call signature");
  assert.equal(signatureRequest.path, "main.zyl");
  assert.equal(signatureRequest.source, source);
  assert.equal(typeof signatureRequest.offset, "number");
  assert.deepEqual(errors, []);
  console.log("Studio shows compiler-resolved parameter help for imported Zelyra functions");
} finally {
  clearTimeout(startupTimer);
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) processHandle.kill("SIGTERM");
  await rm(project, { recursive: true, force: true });
}
