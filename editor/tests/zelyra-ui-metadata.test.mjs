import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { zelyraUiComponentMetadata } from "../src/zelyra-ui-metadata.js";

const source = await readFile(new URL("../../cli/assets/stdlib/ui.zyl", import.meta.url), "utf8");
const sourceComponents = new Map();
const declaration = /pub component ([A-Za-z_][A-Za-z0-9_]*)\s*\{/g;

function matchingBrace(text, opening) {
  let depth = 0;
  for (let index = opening; index < text.length; index += 1) {
    if (text[index] === "{") depth += 1;
    if (text[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  throw new Error("unterminated UI component declaration");
}

for (const match of source.matchAll(declaration)) {
  const opening = match.index + match[0].lastIndexOf("{");
  const closing = matchingBrace(source, opening);
  const body = source.slice(opening + 1, closing);
  const propsStart = body.indexOf("props {");
  let properties = [];
  if (propsStart !== -1) {
    const propsOpening = body.indexOf("{", propsStart);
    const propsClosing = matchingBrace(body, propsOpening);
    properties = [...body.slice(propsOpening + 1, propsClosing).matchAll(/^\s*([A-Za-z_][A-Za-z0-9_]*):/gm)]
      .map((property) => property[1]);
  }
  const namedSlots = [...body.matchAll(/<slot\s+name="([^"]+)"/g)].map((slot) => slot[1]);
  const hasDefaultSlot = /<slot\s*\/>/.test(body);
  sourceComponents.set(match[1], { properties, namedSlots, hasDefaultSlot });
}

assert.deepEqual(
  Object.keys(zelyraUiComponentMetadata).sort(),
  [...sourceComponents.keys()].sort(),
  "Studio metadata must cover every public standard component and no removed components",
);

for (const [name, sourceComponent] of sourceComponents) {
  const metadata = zelyraUiComponentMetadata[name];
  assert.deepEqual(
    metadata.properties.map(({ name: propertyName }) => propertyName).sort(),
    sourceComponent.properties.sort(),
    `${name} property metadata must match the compiler-loaded standard library`,
  );
  for (const slot of sourceComponent.namedSlots) {
    assert.ok(metadata.slots.includes(slot), `${name} must document slot "${slot}"`);
  }
  if (sourceComponent.hasDefaultSlot) {
    assert.ok(
      metadata.slots.some((slot) => slot.startsWith("default") || slot.includes("items")),
      `${name} must document its default slot`,
    );
  }
}

assert.deepEqual(
  zelyraUiComponentMetadata.Button.properties.find(({ name }) => name === "href").defaultValue,
  "nicht gesetzt: echter button type=button",
);
assert.equal(zelyraUiComponentMetadata.PasswordField.properties.some(({ name }) => name === "value"), false);
assert.equal(zelyraUiComponentMetadata.Grid.properties.find(({ name }) => name === "columns").defaultValue, "one");
console.log(`Studio metadata matches ${sourceComponents.size} Zelyra UI components.`);
