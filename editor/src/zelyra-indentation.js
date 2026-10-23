const closingFor = { "(": ")", "[": "]", "{": "}" };

function countIndent(line, context) {
  const length = line.match(/^[\t ]*/)?.[0].length || 0;
  return context.countColumn(line, length);
}

function openBrackets(source, context) {
  const stack = [];
  let blockComment = false;
  let lineIndent = 0;
  let atLineStart = true;
  let quote = "";
  let escaped = false;

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (character === "\n") {
      lineIndent = 0;
      atLineStart = true;
      quote = "";
      escaped = false;
      continue;
    }

    if (atLineStart && character !== "\n") {
      const lineEnd = source.indexOf("\n", index);
      const line = source.slice(index, lineEnd < 0 ? source.length : lineEnd);
      if (line.trim() === "##") {
        blockComment = !blockComment;
        index = lineEnd < 0 ? source.length : lineEnd;
        continue;
      }
    }
    if (blockComment) continue;

    if (quote) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = "";
      atLineStart = false;
      continue;
    }

    if (character === "#" || (character === "/" && source[index + 1] === "/")) {
      const lineEnd = source.indexOf("\n", index);
      index = lineEnd < 0 ? source.length : lineEnd - 1;
      continue;
    }

    if (character === "'" || character === '"') {
      quote = character;
      atLineStart = false;
      continue;
    }

    if (atLineStart && (character === " " || character === "\t")) {
      lineIndent = context.countColumn(source.slice(source.lastIndexOf("\n", index - 1) + 1, index + 1));
      continue;
    }
    atLineStart = false;

    if (closingFor[character]) {
      stack.push({ bracket: character, indent: lineIndent });
    } else if (character === ")" || character === "]" || character === "}") {
      if (stack.at(-1) && closingFor[stack.at(-1).bracket] === character) stack.pop();
    }
  }

  return stack;
}

/** Zelyra uses four-space blocks; strings and comments cannot change block depth. */
export function zelyraIndentation(context, position) {
  const prefix = context.state.doc.sliceString(0, position);
  const stack = openBrackets(prefix, context);
  const nextToken = context.textAfterPos(position).match(/^\s*([}\])])/);
  const top = stack.at(-1);

  if (nextToken && top && closingFor[top.bracket] === nextToken[1]) return top.indent;
  if (top) return top.indent + context.unit;

  const currentPrefix = prefix.slice(prefix.lastIndexOf("\n") + 1);
  return countIndent(currentPrefix, context);
}
