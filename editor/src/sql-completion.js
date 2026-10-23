export function sqlBodyPrefix(source, offset) {
  const limit = Math.min(offset, source.length);
  let index = 0;
  let blockComment = false;
  while (index < limit) {
    const lineStart = source.lastIndexOf("\n", index - 1) + 1;
    const lineEnd = source.indexOf("\n", index);
    const line = source.slice(lineStart, lineEnd < 0 ? source.length : lineEnd).trim();
    if (line === "##") blockComment = !blockComment;
    if (blockComment) {
      index = lineEnd < 0 ? limit : lineEnd + 1;
      continue;
    }
    const char = source[index];
    if (char === "#" || source.startsWith("//", index)) {
      index = lineEnd < 0 ? limit : lineEnd + 1;
      continue;
    }
    if (char === "'" || char === '"') {
      const quote = char;
      index += 1;
      while (index < limit) {
        if (source[index] === "\\") { index += 2; continue; }
        if (source[index] === quote) {
          if (source[index + 1] === quote) { index += 2; continue; }
          index += 1;
          break;
        }
        index += 1;
      }
      continue;
    }
    const identifier = /^[A-Za-z_][\w]*/.exec(source.slice(index));
    if (!identifier) { index += 1; continue; }
    const word = identifier[0];
    const wordStart = index;
    index += word.length;
    if (word !== "sql") continue;
    let header = index;
    while (/\s/.test(source[header] || "") && header < limit) header += 1;
    if (source[header] === "<") {
      let angleDepth = 0;
      for (; header < limit; header += 1) {
        if (source[header] === "<") angleDepth += 1;
        else if (source[header] === ">" && --angleDepth === 0) { header += 1; break; }
      }
    }
    while (/\s/.test(source[header] || "") && header < limit) header += 1;
    if (source[header] !== "{") continue;
    const bodyStart = header + 1;
    let bodyEnd = bodyStart;
    let braceDepth = 0;
    let quote = "";
    while (bodyEnd < source.length) {
      const current = source[bodyEnd];
      if (quote) {
        if (current === quote) {
          if (source[bodyEnd + 1] === quote) { bodyEnd += 2; continue; }
          quote = "";
        }
      } else if (current === "'" || current === '"') quote = current;
      else if (current === "{") braceDepth += 1;
      else if (current === "}") {
        if (braceDepth === 0) break;
        braceDepth -= 1;
      }
      bodyEnd += 1;
    }
    if (offset >= bodyStart && offset <= bodyEnd) return source.slice(bodyStart, offset);
    index = Math.max(wordStart + word.length, bodyEnd + 1);
  }
  return null;
}

export function sqlCompletionOptions(query, map, tableDetail = "table") {
  const tableNodes = (map?.nodes || []).filter((node) => node.kind === "table" && Array.isArray(node.columns));
  const tableCandidates = new Map();
  for (const table of tableNodes) {
    const matches = tableCandidates.get(table.label.toLowerCase()) || [];
    matches.push(table);
    tableCandidates.set(table.label.toLowerCase(), matches);
  }
  const byName = new Map([...tableCandidates].filter(([, matches]) => matches.length === 1).map(([name, matches]) => [name, matches[0]]));
  const tableClause = /\b(?:from|join)\s+[`"]?([A-Za-z_][\w]*)?[`"]?\s*$/i.exec(query);
  const afterResolvedTable = /\b(?:from|join)\s+[`"]?[A-Za-z_][\w]*[`"]?\s+$/i.test(query);
  if (tableClause && !afterResolvedTable) {
    return [...byName.values()].map((table) => ({ label: table.label, type: "class", detail: tableDetail }));
  }
  const aliases = new Map();
  const sqlWords = new Set(["where", "join", "left", "right", "inner", "outer", "cross", "on", "group", "order", "limit", "offset", "having", "union", "set", "values", "returning", "and", "or"]);
  const tableRefs = [...query.matchAll(/\b(?:from|join)\s+[`"]?([A-Za-z_][\w]*)[`"]?(?:\s+as\s+([A-Za-z_][\w]*)|\s+(?!(?:where|join|left|right|inner|outer|cross|on|group|order|limit|offset|having|union|set|values|returning|and|or)\b)([A-Za-z_][\w]*))?/gi)];
  for (const match of tableRefs) {
    const table = byName.get(match[1].toLowerCase());
    if (!table) continue;
    aliases.set(match[1].toLowerCase(), table);
    const alias = match[2] || match[3];
    if (alias && !sqlWords.has(alias.toLowerCase())) aliases.set(alias.toLowerCase(), table);
  }
  const qualifier = /([A-Za-z_][\w]*)\.\s*$/.exec(query)?.[1];
  if (qualifier) {
    const table = aliases.get(qualifier.toLowerCase()) || byName.get(qualifier.toLowerCase());
    return (table?.columns || []).map((name) => ({ label: name, type: "property", detail: `${table.label}.${name}` }));
  }
  if (!tableRefs.length) return [];
  const columnOwners = new Map();
  for (const [, tableName] of tableRefs) {
    const table = byName.get(tableName.toLowerCase());
    for (const column of table?.columns || []) {
      const owners = columnOwners.get(column) || [];
      owners.push(table.label);
      columnOwners.set(column, owners);
    }
  }
  return [...columnOwners].filter(([, owners]) => owners.length === 1)
    .map(([column, owners]) => ({ label: column, type: "property", detail: `${owners[0]}.${column}` }));
}

