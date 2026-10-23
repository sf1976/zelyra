import assert from "node:assert/strict";
import { importPathCompletion, importSignature, importedModuleAlias, importedModuleAliasAtCursor, importedModuleCompletion } from "../src/module-completion.js";

const modules = [
  { alias: "math", symbols: [
    { label: "add", kind: "function", detail: "function" },
    { label: "Money", kind: "class", detail: "record" },
  ] },
  { alias: "invoice", symbols: [{ label: "Invoice", kind: "class", detail: "class" }] },
];

const source = 'import "src/math.zyl" as math\nimport "src/invoice.zyl" as invoice\nfn main() { math:: }';
assert.equal(importedModuleAlias("print(total)", source), null);
assert.equal(importedModuleAlias("math::", source), "math");
assert.equal(importedModuleAlias("Maybe::", source), null, "enum namespaces keep ordinary completion behavior");
const namespacePrefix = source.slice(0, source.indexOf("math::") + "math::".length);
assert.equal(importedModuleAliasAtCursor(namespacePrefix, namespacePrefix.length), "math",
  "typing an imported namespace delimiter resolves its public module");
const namespaceString = `${source}\nlet note = "math::`;
assert.equal(importedModuleAliasAtCursor(namespaceString, namespaceString.length), null,
  "namespace suggestions do not activate inside strings");
const namespaceComment = `${source}\n# math::`;
assert.equal(importedModuleAliasAtCursor(namespaceComment, namespaceComment.length), null,
  "namespace suggestions do not activate inside line comments");
const namespaceBlockComment = `${source}\n##\nmath::`;
assert.equal(importedModuleAliasAtCursor(namespaceBlockComment, namespaceBlockComment.length), null,
  "namespace suggestions do not activate inside block comments");
assert.equal(importedModuleAlias("math::", '##\nimport "src/math.zyl" as math\n##\nfn main() {}'), null,
  "commented imports cannot authorize namespace suggestions");
assert.equal(importSignature(`${source}\nfn main() { math::add(1, 2) }`), 'import "src/math.zyl" as math\nimport "src/invoice.zyl" as invoice');
assert.deepEqual(importedModuleCompletion("math", modules), {
  alias: "math",
  options: [
    { label: "add", type: "function", detail: "function" },
    { label: "Money", type: "class", detail: "record" },
  ],
});
assert.deepEqual(importedModuleCompletion("invoice", modules), {
  alias: "invoice",
  options: [{ label: "Invoice", type: "class", detail: "class" }],
});
assert.equal(importedModuleCompletion(null, modules), null);
assert.deepEqual(importedModuleCompletion("empty", [{ alias: "empty", symbols: [] }]), {
  alias: "empty",
  options: [],
}, "an imported module without resolved declarations must not fall back to global symbols");

const projectFiles = [
  "main.zyl",
  "src/customer.zyl",
  "src/customer_view.zyl",
  "src/invoice.zyl",
  "README.md",
  "src/customer.zyl",
];
const importSource = 'import "src/c\nfn main() {}';
assert.deepEqual(importPathCompletion(importSource, 'import "src/c'.length, projectFiles, "main.zyl"), {
  from: 'import "'.length,
  options: [
    { label: "src/customer_view.zyl", type: "namespace", detail: "customer_view" },
    { label: "src/customer.zyl", type: "namespace", detail: "customer" },
  ],
}, "import path completion only offers matching Zelyra source files and deduplicates the workspace tree");
const emptyImport = 'import "';
assert.deepEqual(importPathCompletion(emptyImport, emptyImport.length, projectFiles, "main.zyl").options.map((option) => option.label), [
  "src/customer_view.zyl", "src/customer.zyl", "src/invoice.zyl",
], "opening an import quote offers project source modules");
const repeatedImport = 'import "src/customer.zyl" as customer\nimport "src/c';
assert.deepEqual(importPathCompletion(repeatedImport, repeatedImport.length, projectFiles, "main.zyl").options.map((option) => option.label), [
  "src/customer_view.zyl",
], "already imported modules are excluded");
const importLikeString = 'let text = "import \\"src/c';
assert.equal(importPathCompletion(importLikeString, importLikeString.length, projectFiles, "main.zyl"), null,
  "import-like text inside a string does not start path completion");
const importLikeComment = '##\nimport "src/c';
assert.equal(importPathCompletion(importLikeComment, importLikeComment.length, projectFiles, "main.zyl"), null,
  "import-like text inside a block comment does not start path completion");
const selfImport = 'import "src/customer.zyl" as customer\nimport "src/i';
assert.equal(importPathCompletion(selfImport, selfImport.length, projectFiles, "src/invoice.zyl").options.some((option) => option.label === "src/invoice.zyl"), false,
  "a source file cannot import itself");

console.log("Zelyra import paths and namespace completion stay within project files and imported exports.");
