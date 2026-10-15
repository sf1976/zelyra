import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { zelyraKeywords } from "../src/zelyra-language.js";

const lexer = await readFile(new URL("../../lexer/src/lib.rs", import.meta.url), "utf8");
const reservedWords = new Set(
  [...lexer.matchAll(/^\s*"([a-z][a-z0-9_]*)"\s*=>\s*TokenKind::/gm)].map((match) => match[1]),
);

assert.deepEqual(
  [...zelyraKeywords].sort(),
  [...reservedWords].sort(),
  "Studio syntax highlighting must match the compiler's reserved words exactly",
);
assert.ok(zelyraKeywords.has("enum"));
assert.ok(zelyraKeywords.has("parallel"));
assert.ok(zelyraKeywords.has("await"));
assert.ok(zelyraKeywords.has("transaction"));
assert.ok(!zelyraKeywords.has("record"), "unreserved words must not be highlighted as keywords");

console.log(`Studio syntax keywords match all ${reservedWords.size} compiler-reserved words.`);
