import assert from "node:assert/strict";
import path from "node:path";
import { chromium } from "playwright";
import { build } from "esbuild";

const projectRoot = path.resolve(import.meta.dirname, "../..");
const bundledEditor = await build({
  stdin: {
    resolveDir: path.join(projectRoot, "editor"),
    sourcefile: "zelyra-editing-browser-entry.js",
    contents: `
      import { defaultKeymap } from "@codemirror/commands";
      import { EditorState } from "@codemirror/state";
      import { EditorView, keymap } from "@codemirror/view";
      import { zelyraLanguage } from "./src/zelyra-language.js";
      import { zelyraEditingExtensions, zelyraEditingKeymap } from "./src/zelyra-editing.js";
      window.mountZelyraEditor = (parent) => new EditorView({
        parent,
        state: EditorState.create({
          doc: "",
          extensions: [zelyraLanguage, ...zelyraEditingExtensions, keymap.of([...zelyraEditingKeymap, ...defaultKeymap])],
        }),
      });
    `,
  },
  bundle: true,
  format: "iife",
  platform: "browser",
  target: "es2020",
  write: false,
});

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.setContent('<main id="editor"></main>');
  await page.addScriptTag({ content: bundledEditor.outputFiles[0].text });
  await page.evaluate(() => { window.zelyraEditor = window.mountZelyraEditor(document.querySelector("#editor")); });
  const editor = page.locator(".cm-content[contenteditable=true]");
  await editor.focus();
  await page.keyboard.type("fn main() {");
  assert.equal(await page.evaluate(() => window.zelyraEditor.state.doc.toString()), "fn main() {}", "opening a block brace inserts its close");
  await page.keyboard.press("Enter");
  assert.equal(await page.evaluate(() => window.zelyraEditor.state.doc.toString()), "fn main() {\n    \n}", "Enter indents a block body by four spaces and aligns its close");
  await page.keyboard.press("Tab");
  assert.equal(await page.evaluate(() => window.zelyraEditor.state.doc.toString()), "fn main() {\n        \n}", "Tab indents by one four-space unit");
  await page.keyboard.press("Shift+Tab");
  assert.equal(await page.evaluate(() => window.zelyraEditor.state.doc.toString()), "fn main() {\n    \n}", "Shift+Tab removes one indentation unit");
  await page.evaluate(() => {
    const source = "let title = ";
    window.zelyraEditor.dispatch({ changes: { from: 0, to: window.zelyraEditor.state.doc.length, insert: source }, selection: { anchor: source.length } });
  });
  await page.keyboard.type('"');
  assert.equal(await page.evaluate(() => window.zelyraEditor.state.doc.toString()), 'let title = ""', "quotation marks are paired automatically");
  console.log("Zelyra Studio browser editing passed: paired brackets/quotes, block indentation, Tab, and Shift+Tab.");
} finally {
  await browser.close();
}
