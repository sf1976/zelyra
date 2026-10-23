import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { indentLess, indentWithTab } from "@codemirror/commands";
import { indentOnInput, indentService, indentUnit } from "@codemirror/language";
import { EditorState } from "@codemirror/state";
import { zelyraIndentation } from "./zelyra-indentation.js";

/** CodeMirror behavior shared by Zelyra Studio and its editor interaction tests. */
export const zelyraEditingExtensions = [
  closeBrackets(),
  indentUnit.of("    "),
  EditorState.tabSize.of(4),
  indentService.of(zelyraIndentation),
  indentOnInput(),
];

export const zelyraEditingKeymap = [
  ...closeBracketsKeymap,
  indentWithTab,
  { key: "Shift-Tab", run: indentLess },
];
