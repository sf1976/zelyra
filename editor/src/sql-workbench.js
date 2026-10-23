import { autocompletion } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { HighlightStyle, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
import { EditorState } from "@codemirror/state";
import { tags } from "@lezer/highlight";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { sqlCompletionOptions } from "./sql-completion.js";

const keywords = new Set((
  "select from where join left right inner outer cross on as and or not null is in like between " +
  "group by order having limit offset distinct union all with recursive case when then else end " +
  "count sum avg min max coalesce cast date interval asc desc true false exists insert update delete " +
  "create alter drop table view primary key foreign references index explain show describe"
).split(/\s+/));

const sqlLanguage = StreamLanguage.define({
  startState: () => ({}),
  token(stream) {
    if (stream.eatSpace()) return null;
    if (stream.match("--") || stream.match("#")) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.match("/*")) {
      while (!stream.eol()) {
        if (stream.match("*/")) break;
        stream.next();
      }
      return "comment";
    }
    const quote = stream.peek();
    if (["'", '"', "`"].includes(quote)) {
      stream.next();
      while (!stream.eol()) {
        if (stream.next() === quote) {
          if (stream.peek() !== quote) break;
          stream.next();
        }
      }
      return quote === "`" ? "variableName" : "string";
    }
    if (stream.match(/^[0-9]+(?:\.[0-9]+)?/)) return "number";
    if (stream.match(/^[A-Za-z_][\w$]*/)) {
      const word = stream.current().toLowerCase();
      return keywords.has(word) ? "keyword" : "variableName";
    }
    if (stream.match(/^(?:<=|>=|<>|!=|::|\|\||[-+*/%=<>])/)) return "operator";
    stream.next();
    return null;
  },
});

const sqlHighlighting = syntaxHighlighting(HighlightStyle.define([
  { tag: tags.keyword, color: "var(--token-keyword)", fontWeight: "700" },
  { tag: tags.string, color: "var(--token-string)" },
  { tag: tags.number, color: "var(--token-number)" },
  { tag: tags.comment, color: "var(--token-comment)", fontStyle: "italic" },
  { tag: tags.operator, color: "var(--token-operator)" },
  { tag: tags.variableName, color: "var(--token-name)" },
]));

const sqlKeywords = ["SELECT", "FROM", "WHERE", "JOIN", "LEFT JOIN", "INNER JOIN", "ON", "GROUP BY", "ORDER BY", "LIMIT", "COUNT(*)"];

export function createSqlWorkbenchEditor(parent, { initial = "", getTables, run, onChange }) {
  const completeSql = (context) => {
    const word = context.matchBefore(/[A-Za-z_][\w$]*/);
    if (!word && !context.explicit) return null;
    const query = context.state.doc.sliceString(0, context.pos);
    const tables = (getTables() || []).map((table) => {
      const columns = Array.isArray(table.live_columns) && table.live_columns.length
        ? table.live_columns
        : (table.declared_columns || []);
      return {
        kind: "table",
        label: table.name,
        columns: columns.map((column) => typeof column === "string" ? column : column.name),
      };
    });
    return {
      from: word?.from ?? context.pos,
      options: [
        ...sqlKeywords.map((label) => ({ label, type: "keyword" })),
        ...sqlCompletionOptions(query, { nodes: tables }, "Zelyra-Tabelle"),
      ],
      validFor: /^[A-Za-z_][\w$]*$/,
    };
  };
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc: initial,
      extensions: [
        lineNumbers(),
        history(),
        keymap.of([
          { key: "Mod-Enter", run: () => { run(); return true; } },
          ...historyKeymap,
          ...defaultKeymap,
        ]),
        sqlLanguage,
        sqlHighlighting,
        autocompletion({ override: [completeSql], activateOnTyping: true }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onChange?.(update.state.doc.toString());
        }),
        EditorView.lineWrapping,
        EditorView.theme({
          "&": { height: "100%", backgroundColor: "var(--editor-bg)", color: "var(--ink)" },
          ".cm-scroller": { overflow: "auto", fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace", fontSize: "12px", lineHeight: "1.65" },
          ".cm-gutters": { backgroundColor: "var(--editor-gutter)", color: "var(--faint)", border: "none" },
          ".cm-activeLineGutter, .cm-activeLine": { backgroundColor: "var(--editor-active)" },
          ".cm-content": { caretColor: "var(--accent)" },
          ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
          ".cm-selectionBackground, ::selection": { backgroundColor: "var(--selection)" },
        }),
      ],
    }),
  });
  return {
    view,
    getValue: () => view.state.doc.toString(),
    setValue: (value) => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } }),
    focus: () => view.focus(),
  };
}
