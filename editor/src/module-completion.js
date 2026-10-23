export function importedModuleAlias(prefix, source) {
  const match = /(?:^|[^\w])([A-Za-z_][\w]*)::\s*$/.exec(prefix);
  if (!match) return null;

  const alias = match[1];
  let inBlockComment = false;
  for (const line of source.split(/\r?\n/)) {
    if (line.trim() === "##") {
      inBlockComment = !inBlockComment;
      continue;
    }
    if (inBlockComment || /^\s*(?:#|\/\/)/.test(line)) continue;
    const imported = /^\s*import\s+"[^"]+"\s+as\s+([A-Za-z_][\w]*)\s*$/.exec(line);
    if (imported?.[1] === alias) return alias;
  }
  return null;
}

export function importedModuleAliasAtCursor(source, cursor) {
  const beforeCursor = source.slice(0, cursor);
  const lines = beforeCursor.split(/\r?\n/);
  let inBlockComment = false;
  for (const line of lines.slice(0, -1)) {
    if (line.trim() === "##") inBlockComment = !inBlockComment;
  }
  if (inBlockComment) return null;

  const currentLine = lines.at(-1) || "";
  let quote = null;
  let escaped = false;
  for (let index = 0; index < currentLine.length; index += 1) {
    const character = currentLine[index];
    if (quote) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "#" || currentLine.startsWith("//", index)) return null;
    if (character === "\"" || character === "'") quote = character;
  }
  if (quote) return null;

  const match = /(?:^|[^\w])([A-Za-z_][\w]*)::$/.exec(beforeCursor);
  return match ? importedModuleAlias(`${match[1]}::`, source) : null;
}

export function importedModuleCompletion(alias, modules) {
  if (!alias) return null;
  const module = modules.find((item) => item.alias === alias);
  if (!module) return null;
  return {
    alias,
    options: (module.symbols || []).map((symbol) => ({
      label: symbol.label,
      type: symbol.kind,
      detail: symbol.detail || "module member",
    })),
  };
}

export function importPathCompletion(source, cursor, files, currentPath = "") {
  const beforeCursor = source.slice(0, cursor);
  const lineStart = beforeCursor.lastIndexOf("\n") + 1;
  const currentLine = beforeCursor.slice(lineStart);
  const match = /^\s*import\s+"([^\"]*)$/.exec(currentLine);
  if (!match) return null;

  const inBlockComment = beforeCursor.slice(0, lineStart).split(/\r?\n/).reduce((inside, line) => {
    return line.trim() === "##" ? !inside : inside;
  }, false);
  if (inBlockComment) return null;

  const prefix = match[1];
  const importedPaths = new Set(
    [...source.matchAll(/^\s*import\s+"([^\"]+)"\s+as\s+[A-Za-z_]\w*\s*$/gm)]
      .map((item) => item[1]),
  );
  const options = [...new Set(files)]
    .filter((path) => typeof path === "string" && path.endsWith(".zyl"))
    .filter((path) => path !== currentPath && !importedPaths.has(path) && path.startsWith(prefix))
    .sort((left, right) => left.localeCompare(right))
    .map((path) => ({
      label: path,
      type: "namespace",
      detail: path.slice(path.lastIndexOf("/") + 1, -4),
    }));

  return { from: cursor - prefix.length, options };
}

export function importSignature(source) {
  return source.split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => /^import\s+/.test(line))
    .join("\n");
}
