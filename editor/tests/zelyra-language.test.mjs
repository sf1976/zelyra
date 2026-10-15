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

for (const grammarPath of [
  new URL("../../editors/syntaxes/zelyra.tmLanguage.json", import.meta.url),
  new URL("../../editors/zelyra.tmLanguage.json", import.meta.url),
]) {
  const grammar = JSON.parse(await readFile(grammarPath, "utf8"));
  const keywordPattern = grammar.repository.keywords.patterns[0]?.match;
  const keywordGroup = keywordPattern?.match(/^\\b\((.*)\)\\b$/)?.[1];
  assert.ok(keywordGroup, `${grammarPath} has a parseable reserved-word rule`);
  assert.deepEqual(
    keywordGroup.split("|").sort(),
    [...reservedWords].sort(),
    `${grammarPath} reserved words match the compiler lexer`,
  );
  assert.ok(grammar.repository.operators.patterns.some((pattern) => pattern.match.includes("\\|>")), `${grammarPath} highlights the pipeline operator`);
  assert.ok(grammar.repository.operators.patterns.some((pattern) => pattern.match.includes("\\.\\.")), `${grammarPath} highlights record spread`);
  assert.ok(!JSON.stringify(grammar.repository.strings).includes("interpolation"), `${grammarPath} does not advertise unsupported interpolation`);
}

console.log(`Studio syntax keywords match all ${reservedWords.size} compiler-reserved words.`);
