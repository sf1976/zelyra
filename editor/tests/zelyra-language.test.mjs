import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { getIndentation, indentUnit, IndentContext, indentService } from "@codemirror/language";
import { EditorState } from "@codemirror/state";
import { zelyraKeywords, zelyraLanguage } from "../src/zelyra-language.js";
import { zelyraIndentation } from "../src/zelyra-indentation.js";

function indentation(source, position, options = {}) {
  const state = EditorState.create({
    doc: source,
    extensions: [indentUnit.of("    "), EditorState.tabSize.of(4), indentService.of(zelyraIndentation)],
  });
  const context = new IndentContext(state, options);
  return getIndentation(context, position);
}

const languageState = EditorState.create({ doc: "", extensions: [zelyraLanguage] });
assert.deepEqual(languageState.languageDataAt("closeBrackets", 0)[0].brackets, ["(", "[", "{", "'", '"']);
const indentRule = languageState.languageDataAt("indentOnInput", 0)[0];
assert.ok(indentRule instanceof RegExp, "CodeMirror indentOnInput language data must be a RegExp");
assert.ok(indentRule.test("    }"), "typing a closing brace triggers Zelyra indentation");

const nestedBlocks = "fn main() {\n    if true {\n        \n    }\n}";
assert.equal(indentation(nestedBlocks, nestedBlocks.indexOf("        \n") + 8), 8, "nested Zelyra blocks indent by four spaces each");
const closingBrace = "fn main() {\n    }";
assert.equal(indentation(closingBrace, closingBrace.indexOf("\n") + 1), 0, "a closing brace returns to its opener's indentation");
const stringBrace = 'let text = "{ not a block }"\nfn main() {\n';
assert.equal(indentation(stringBrace, stringBrace.length), 4, "braces inside strings do not affect indentation");
const commentedBrace = "# { ignored comment\nfn main() {\n";
assert.equal(indentation(commentedBrace, commentedBrace.length), 4, "braces inside line comments do not affect indentation");
const braceDocument = "fn main() {";
assert.equal(
  indentation(braceDocument, braceDocument.length, { simulateBreak: braceDocument.length, simulateDoubleBreak: true }),
  4,
  "Enter between an auto-paired block brace starts a four-space body",
);

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
