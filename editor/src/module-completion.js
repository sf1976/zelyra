export function importedModuleAlias(prefix, source) {
  const match = /(?:^|[^\w])([A-Za-z_][\w]*)::\s*$/.exec(prefix);
  if (!match) return null;

  const alias = match[1];
  const importedAliases = [...source.matchAll(/^\s*import\s+"[^"]+"\s+as\s+([A-Za-z_][\w]*)\s*$/gm)];
  return importedAliases.some((item) => item[1] === alias) ? alias : null;
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
