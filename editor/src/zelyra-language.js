import { StreamLanguage } from "@codemirror/language";

// Keep aligned with the reserved words in lexer/src/lib.rs; the editor test
// compares this set with the compiler's keyword mapping.
export const zelyraKeywords = new Set((
  "import pub fn type enum struct database table engine postgres primary auto required default index unique " +
  "return if else for in while invariant loop break continue mutable match sql page tableview view component props html " +
  "form crud auth api input output errors version deprecated rate_limit login_rate_limit login_block_seconds " +
  "reset_tokens reset_rate_limit reset_block_seconds handler requires permits sessions permissions roles " +
  "role_permissions audit audit_chain admin_path admin_permission admin_role title layout list detail search sort " +
  "sortable searchable paginated source columns filter soft_delete field action label icon placeholder max widget " +
  "appearance readonly fields success redirect confirm confirm_page success_page error_page transaction parallel await " +
  "uses ensures true false"
).split(/\s+/));

export const zelyraLanguage = StreamLanguage.define({
  startState: () => ({ inBlockComment: false }),
  languageData: {
    commentTokens: { line: "#", block: { open: "##", close: "##" } },
    closeBrackets: { brackets: ["(", "[", "{", "'", '"'] },
    indentOnInput: /^\s*[}\])\]]/,
  },
  token(stream, state) {
    if (stream.string.trim() === "##") {
      stream.skipToEnd();
      state.inBlockComment = !state.inBlockComment;
      return "comment";
    }
    if (state.inBlockComment) {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.eatSpace()) return null;
    if (stream.match("#")) { stream.skipToEnd(); return "comment"; }
    if (stream.match("//")) { stream.skipToEnd(); return "comment"; }
    if (stream.match(/(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')/)) return "string";
    if (stream.match(/\b\d+(?:\.\d+)?\b/)) return "number";
    if (stream.match(/[A-Za-z_][\w]*/)) {
      const word = stream.current();
      if (zelyraKeywords.has(word)) return "keyword";
      if (/^[A-Z]/.test(word)) return "typeName";
      return "variableName";
    }
    if (stream.match(/[{}()[\],.;:]/)) return "punctuation";
    if (stream.match(/[=<>!+*/%&|?-]+/)) return "operator";
    stream.next();
    return null;
  },
});
