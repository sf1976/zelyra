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

export function importSignature(source) {
  return source.split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => /^import\s+/.test(line))
    .join("\n");
}
