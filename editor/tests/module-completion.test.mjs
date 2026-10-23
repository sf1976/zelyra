import assert from "node:assert/strict";
import { importSignature, importedModuleAlias, importedModuleCompletion } from "../src/module-completion.js";

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

console.log("Zelyra namespace completion stays within imported module exports.");
