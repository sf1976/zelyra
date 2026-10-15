import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-canvas-"));
const binary = resolve(process.env.ZELYRA_BIN || "../target/debug/zelyra");
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
let processHandle;
let browser;

try {
  await writeFile(join(project, "main.zyl"), `component Banner {
    html { <aside><slot /></aside> }
}
view Shell {
    html { <html><body><main><Banner><slot /></Banner></main></body></html> }
}
page "/reports/{id}" {
    view: Shell
    input { term: String }
    html { <h1>{term}</h1><script>document.body.dataset.executed = "true"</script> }
}
fn main() {}
`);
  await writeFile(join(project, "zelyra.theme.css"), `:where(html[data-z-ui]) {
  --z-color-primary: #23654f;
  --z-color-on-primary: #ffffff;
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
  await page.locator("#canvas-button").click();
  await page.waitForFunction(() => document.querySelector("#canvas-source")?.disabled === false);
  await page.locator("#canvas-target").selectOption({ label: "page · /reports/{id}" });
  await page.locator("#canvas-sample").fill('{"term":"<Acme & Co>"}');
  await page.locator("#canvas-render").click();
  await page.waitForFunction(() => document.querySelector("#canvas-source")?.disabled === false);
  const preview = page.frameLocator("#canvas-frame");
  await preview.locator("h1").waitFor();
  assert.equal(await preview.locator("h1").textContent(), "<Acme & Co>");
  assert.equal(await preview.locator("script").count(), 1, "preview keeps project markup visible for inspection");
  assert.equal(await preview.locator("body").getAttribute("data-executed"), null, "preview must not execute project scripts");
  await page.locator("#canvas-component-target").selectOption({ label: "Banner" });
  await page.locator("#canvas-component-source").click();
  await page.waitForFunction(() => document.querySelector(".cm-activeLine")?.textContent?.includes("component Banner"));
  assert.match(await page.locator(".cm-activeLine").textContent(), /component Banner/);

  const primaryToken = await page.locator("#canvas-theme-token").evaluate((select) =>
    [...select.options].find((option) => option.textContent.includes("--z-color-primary"))?.value,
  );
  assert.notEqual(primaryToken, undefined, "theme editor should list the declared primary color");
  await page.locator("#canvas-theme-token").selectOption(primaryToken);
  await page.locator("#canvas-theme-color").evaluate((input) => {
    input.value = "#aabbcc";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await page.locator("#canvas-theme-apply").click();
  await page.waitForFunction(() => document.querySelector("#canvas-theme-status")?.textContent?.includes("geändert"));
  const themeSource = await page.locator(".cm-content").textContent();
  assert.match(themeSource, /--z-color-primary:\s*#aabbcc/);
  assert.match(themeSource, /--z-color-on-primary:\s*#000000/);
  await page.locator(".cm-content[contenteditable=true]").click();
  await page.keyboard.press("Control+z");
  assert.match(await page.locator(".cm-content").textContent(), /--z-color-primary:\s*#23654f/);
  assert.match(await page.locator(".cm-content").textContent(), /--z-color-on-primary:\s*#ffffff/);
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio Canvas rendered escaped sample data, blocked project scripts, and applied/undid theme edits in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
}
