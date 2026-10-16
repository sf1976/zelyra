import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { spawn, spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, firefox, webkit } from "playwright";
import axe from "axe-core";

const databaseUrl = process.env.ZELYRA_DB_TIMEOUT_TEST_URL;
assert.ok(databaseUrl, "set ZELYRA_DB_TIMEOUT_TEST_URL to a disposable MariaDB database");
const database = new URL(databaseUrl);
assert.equal(database.protocol, "mariadb:", "the outbox E2E fixture must use a MariaDB URL");
assert.equal(database.hostname, "127.0.0.1", "the test fixture must use loopback only");
const project = await mkdtemp(join(tmpdir(), "zelyra-studio-outbox-"));
const defaultBinary = existsSync(resolve("../target/debug/zelyra"))
  ? resolve("../target/debug/zelyra")
  : existsSync(resolve("target/debug/zelyra"))
    ? resolve("target/debug/zelyra")
    : resolve("../target/debug/zelyra");
const binary = resolve(process.env.ZELYRA_BIN || defaultBinary);
const engineName = process.env.ZELYRA_BROWSER_ENGINE || "chromium";
const engine = { chromium, firefox, webkit }[engineName];
assert.ok(engine, `unknown browser engine: ${engineName}`);
const eventId = `studio-outbox-${process.pid}-${Date.now()}`;
const secretBody = "this private mail body must never appear in Studio";
let processHandle;
let browser;

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  assert.equal(result.status, 0, `${command} failed: ${result.stderr}`);
  return result.stdout;
}

function mariadb(args, sql) {
  const env = { ...process.env, MYSQL_PWD: decodeURIComponent(database.password) };
  return run("mariadb", [
    "--protocol=tcp", "-h", database.hostname, "-P", database.port || "3306",
    "-u", decodeURIComponent(database.username), database.pathname.slice(1), "-e", sql,
  ], { env });
}

try {
  await writeFile(join(project, "main.zyl"), "database main { engine: mariadb }\nfn main() {}\n");
  await writeFile(join(project, ".env"), `DATABASE_URL=${databaseUrl}\n`);
  run(binary, ["outbox", "setup", "main.zyl"], {
    cwd: project,
    env: { ...process.env, DATABASE_URL: databaseUrl },
  });
  const payload = JSON.stringify({
    to: "outbox-recipient@example.test",
    subject: "private subject",
    text: secretBody,
    attachments: [{ filename: "invoice-studio.pdf", content_type: "application/pdf", content: "JVBERi0xLjQ=" }],
  }).replaceAll("'", "''");
  const escapedId = eventId.replaceAll("'", "''");
  mariadb([], `INSERT INTO _zelyra_outbox (event_id,event_type,payload,created_at,available_at,attempts,last_error,exhausted_at) VALUES ('${escapedId}','zelyra.mail.send','${payload}',UTC_TIMESTAMP(6),UTC_TIMESTAMP(6),8,'transport unavailable',UTC_TIMESTAMP(6))`);

  const port = await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const selected = server.address().port;
      server.close(() => resolvePort(selected));
    });
  });
  const childEnv = { ...process.env };
  childEnv.DATABASE_URL = databaseUrl;
  processHandle = spawn(binary, ["editor", project, "--no-open", "--port", String(port)], { env: childEnv, stdio: ["ignore", "pipe", "pipe"] });
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
  page.on("dialog", async (dialog) => {
    assert.match(dialog.message(), /No email will be sent|jetzt keine E-Mail versendet/i);
    await dialog.accept();
  });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.locator('.tree-item.file[title="main.zyl"]').click();
  await page.locator(".cm-content[contenteditable=true]").waitFor();
  await page.locator("#outbox-button").click();
  const row = page.locator("#outbox-rows tr").filter({ hasText: eventId });
  await row.waitFor();
  assert.match(await row.textContent(), /outbox-recipient@example\.test/);
  assert.match(await row.textContent(), /invoice-studio\.pdf/);
  assert.match(await row.textContent(), /unknown/, "outbox error details should be safely redacted");
  assert.ok(!(await page.locator("#outbox-dialog").textContent()).includes(secretBody));

  const sameOrigin = new URL(url).origin;
  await page.route(`${sameOrigin}/__zelyra/axe-test.js`, (route) => route.fulfill({
    status: 200, contentType: "text/javascript", body: axe.source,
  }));
  await page.addScriptTag({ url: `${sameOrigin}/__zelyra/axe-test.js` });
  const accessibility = await page.evaluate(() => axe.run(document, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"] },
  }));
  assert.deepEqual(accessibility.violations.map(({ id, nodes }) => ({ id, targets: nodes.map(({ target }) => target) })), []);

  await row.getByRole("button", { name: /Erneut einreihen|Requeue/i }).click();
  await page.waitForFunction((id) => {
    const row = [...document.querySelectorAll("#outbox-rows tr")].find((element) => element.textContent.includes(id));
    return row?.textContent.includes("pending") || row?.textContent.includes("ausstehend");
  }, eventId);
  const state = mariadb([], `SELECT delivered_at IS NULL, exhausted_at IS NULL, attempts FROM _zelyra_outbox WHERE event_id='${escapedId}'`)
    .trim().split(/\s+/).slice(-3);
  assert.deepEqual(state, ["1", "1", "0"], "requeue must not deliver the message");
  assert.match(await page.locator("#outbox-status").textContent(), /Ereignis erneut eingereiht|Event requeued/);
  assert.deepEqual(errors, [], `browser errors: ${errors.join(", ")}`);
  console.log(`Studio Outbox showed metadata only and requeued without delivery in ${engineName}.`);
} finally {
  await browser?.close();
  if (processHandle && processHandle.exitCode === null) {
    processHandle.kill("SIGTERM");
    await new Promise((resolveExit) => processHandle.once("exit", resolveExit));
  }
  await rm(project, { recursive: true, force: true });
  try { mariadb([], `DELETE FROM _zelyra_outbox WHERE event_id='${eventId.replaceAll("'", "''")}'`); } catch {}
}
