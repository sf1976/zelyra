export function isTopLevelBlankPosition(source, position) {
  const lineStart = position === 0 ? 0 : source.lastIndexOf("\n", position - 1) + 1;
  const lineEnd = source.indexOf("\n", position);
  if (source.slice(lineStart, lineEnd < 0 ? source.length : lineEnd).trim() !== "") return false;

  let depth = 0;
  let quote = null;
  let escaped = false;
  let lineComment = false;
  let blockComment = false;
  for (let index = 0; index < position; index += 1) {
    const character = source[index];
    const next = source[index + 1];
    if (lineComment) {
      if (character === "\n") lineComment = false;
      continue;
    }
    if (blockComment) {
      if (character === "#" && next === "#") {
        blockComment = false;
        index += 1;
      }
      continue;
    }
    if (quote) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "#" && next === "#") {
      blockComment = true;
      index += 1;
    } else if (character === "#" || (character === "/" && next === "/")) {
      lineComment = true;
      if (character === "/") index += 1;
    } else if (character === '"' || character === "'") {
      quote = character;
    } else if (character === "{") {
      depth += 1;
    } else if (character === "}") {
      depth = Math.max(0, depth - 1);
    }
  }
  return depth === 0 && quote === null && !blockComment;
}

function crudName(tableName) {
  return tableName
    .split(/[_-]+/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join("");
}

export function nextCrudCompletions(map, language = "en") {
  if (map?.incomplete || !Array.isArray(map?.nodes) || !Array.isArray(map?.edges)) return [];
  const tablesByName = new Map();
  for (const node of map.nodes) {
    if (node.kind !== "table" || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(node.label || "")) continue;
    const group = tablesByName.get(node.label) || [];
    group.push(node);
    tablesByName.set(node.label, group);
  }
  const usedTables = new Set(map.edges.filter((edge) => edge.label === "uses table").map((edge) => edge.to));
  const suggestions = [];
  for (const [tableName, tables] of tablesByName) {
    if (tables.length !== 1 || usedTables.has(tables[0].id)) continue;
    const name = crudName(tableName);
    if (!name || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) continue;
    const code = `crud ${name} -> ${tableName} {}`;
    suggestions.push({
      label: code,
      apply: code,
      type: "keyword",
      detail: language === "de" ? `CRUD-Block für die Tabelle ${tableName}` : `CRUD block for the declared ${tableName} table`,
    });
  }
  return suggestions;
}
