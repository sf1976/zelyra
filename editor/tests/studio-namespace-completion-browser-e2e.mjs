import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-module-completion-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const engine = await chromium.launch({ headless: true });
let processHandle;

try {
  await mkdir(join(project, "src"));
  await writeFile(join(project, "main.zyl"), 'import "src/math.zyl" as math\nfn main() {\n    \n}\n');
  await writeFile(join(project, "src/math.zyl"),
    "pub fn add(left: Int, right: Int) -> Int { return left + right }\nfn private_helper() -> Int { return 0 }\n");
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
  const page = await engine.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  const editor = page.locator(".cm-content[contenteditable=true]");
  await editor.waitFor();
  await editor.focus();
  await page.keyboard.press("Control+End");
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("ArrowUp");
  await page.keyboard.press("Home");
  await page.keyboard.type("    math::");
  await page.locator(".cm-tooltip-autocomplete").waitFor();
  await page.waitForFunction(() => [...document.querySelectorAll(".cm-tooltip-autocomplete [role=option]")]
    .some((option) => option.textContent?.includes("add")));
  const options = await page.locator(".cm-tooltip-autocomplete [role=option]").allTextContents();
  assert.ok(options.some((option) => option.includes("add")), `public export missing: ${options.join(", ")}`);
  assert.ok(!options.some((option) => option.includes("private_helper")), "private module declarations must stay hidden");
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log("Studio opened imported module exports automatically after `math::` and kept private members hidden.");
} finally {
  await engine.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
