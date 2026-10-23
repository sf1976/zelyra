import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer as createHttpServer } from "node:http";
import { createServer as createTcpServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const project = await mkdtemp(join(tmpdir(), "zelyra-studio-ai-"));
const source = 'fn main() { print("selected passage") }\nfn hidden_marker() { print("must not be sent") }\n';
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : resolve("target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
let processHandle;
let browser;
let studioTimer;
let received = [];

async function freePort() {
  return new Promise((resolvePort, reject) => {
    const socket = createTcpServer();
    socket.once("error", reject);
    socket.listen(0, "127.0.0.1", () => {
      const port = socket.address().port;
      socket.close(() => resolvePort(port));
    });
  });
}

const provider = createHttpServer(async (request, response) => {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  received.push(JSON.parse(Buffer.concat(chunks).toString("utf8")));
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify({ choices: [{ message: { content: "# geprüfter Vorschlag" } }] }));
});

try {
  await writeFile(join(project, "zelyra.toml"), '[project]\nname = "studio-ai-e2e"\n');
  await writeFile(join(project, "main.zyl"), source);
  const providerPort = await new Promise((resolvePort, reject) => {
    provider.once("error", reject);
    provider.listen(0, "127.0.0.1", () => resolvePort(provider.address().port));
  });
  const studioPort = await freePort();
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(studioPort)], {
    env: {
      ...process.env,
      ZELYRA_STUDIO_AI_ENDPOINT: `http://127.0.0.1:${providerPort}/v1/chat/completions`,
      ZELYRA_STUDIO_AI_MODEL: "local-test-model",
    },
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
    new Promise((_, reject) => { studioTimer = setTimeout(() => reject(new Error("Studio startup timed out")), 10_000); }),
  ]);
  clearTimeout(studioTimer);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  const editor = page.locator(".cm-content[contenteditable=true]");
  await editor.waitFor();
  await editor.click();
  await page.keyboard.press("Control+Home");
  await page.keyboard.press("Shift+End");
  await page.locator('[data-studio-menu="help"]').click();
  await page.getByRole("menuitem", { name: "KI-Vorschlag für Auswahl" }).click();
  await page.locator("#ai-assistant-dialog[open]").waitFor();
  assert.equal(await page.locator("#ai-assistant-selection").inputValue(), source.split("\n")[0]);
  await page.locator("#ai-assistant-instruction").fill("Erkläre die Auswahl kurz.");
  assert.equal(received.length, 0, "opening and reviewing the dialog must not contact the provider");
  await page.locator("#ai-assistant-send").click();
  await page.locator("#ai-assistant-result:not([hidden])").waitFor();
  assert.equal(received.length, 1, "an explicit send makes one provider request");
  const requestText = received[0].messages[1].content;
  assert.match(requestText, /selected passage/);
  assert.doesNotMatch(requestText, /hidden_marker|must not be sent/);
  assert.equal(await page.locator("#ai-assistant-result").textContent(), "# geprüfter Vorschlag");
  assert.equal(await page.locator("#save-button").isDisabled(), true, "suggestion is not applied to source");
  assert.deepEqual(pageErrors, []);
  console.log("Studio AI sends only the reviewed selection after an explicit click and never applies the suggestion");
} finally {
  clearTimeout(studioTimer);
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) processHandle.kill("SIGTERM");
  if (provider.listening) await new Promise((resolveClose) => provider.close(() => resolveClose()));
  await rm(project, { recursive: true, force: true });
}
