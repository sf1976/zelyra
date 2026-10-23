export function databaseRecordEnumOptions(sqlType) {
  const source = String(sqlType || "").trim();
  const match = /^ENUM\s*\(/i.exec(source);
  if (!match || !source.endsWith(")")) return [];
  const values = [];
  let index = match[0].length;
  const end = source.length - 1;
  while (index < end) {
    while (/\s/.test(source[index] || "")) index += 1;
    if (source[index] !== "'") return [];
    index += 1;
    let value = "";
    let closed = false;
    while (index < end) {
      const character = source[index];
      if (character === "\\" && index + 1 < end) {
        value += source[index + 1];
        index += 2;
      } else if (character === "'" && source[index + 1] === "'") {
        value += "'";
        index += 2;
      } else if (character === "'") {
        index += 1;
        closed = true;
        break;
      } else {
        value += character;
        index += 1;
      }
    }
    if (!closed) return [];
    values.push(value);
    while (/\s/.test(source[index] || "")) index += 1;
    if (index === end) break;
    if (source[index] !== ",") return [];
    index += 1;
  }
  return values;
}

export function databaseRecordInputKind(sqlType) {
  const type = String(sqlType || "").trim();
  if (/^(?:BOOL(?:EAN)?\b|BIT\s*\(\s*1\s*\)|TINYINT\s*\(\s*1\s*\))/i.test(type)) return "boolean";
  if (/\bJSONB?\b/i.test(type)) return "json";
  if (databaseRecordEnumOptions(type).length) return "enum";
  if (/^DATE(?:\s|$)/i.test(type) && !/^DATETIME\b/i.test(type)) return "date";
  if (/^TIME(?:\s|\(|$)/i.test(type) && !/\bZONE\b/i.test(type)) return "time";
  if (/^(?:TINYINT|SMALLINT|MEDIUMINT|INT|INTEGER|BIGINT|DECIMAL|NUMERIC|FLOAT|DOUBLE|REAL)\b/i.test(type)) return "number";
  return "text";
}

export function databaseRecordInputValue(kind, value) {
  if (kind === "boolean") return value === true || value === 1 || ["1", "true", "TRUE"].includes(value);
  if (kind === "json") {
    if (value == null || value === "") return "";
    if (typeof value === "string") {
      try { return JSON.stringify(JSON.parse(value), null, 2); }
      catch { return value; }
    }
    return JSON.stringify(value, null, 2);
  }
  return value == null ? "" : String(value);
}

export function parseDatabaseRecordInput(kind, value) {
  if (kind === "boolean") return value === true || value === 1 || ["1", "true", "TRUE"].includes(value);
  if (kind === "json") return JSON.parse(value);
  return value;
}
