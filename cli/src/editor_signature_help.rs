use super::*;
use zelyra_lexer::{lex, Token, TokenKind};

#[derive(Clone, Copy)]
struct CallContext {
    callee_start: usize,
    active_parameter: usize,
}

enum Delimiter {
    Call { open_index: usize, commas: usize },
    Other(char),
}

pub(super) fn signature_help(
    state: &EditorState,
    request: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = request
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| path.ends_with(".zyl"))
        .ok_or((400, "choose a Zelyra source file".into()))?;
    let _path = safe_editor_path(&state.root, relative, false)?;
    let source = request
        .get("source")
        .and_then(Value::as_str)
        .ok_or((400, "source is required".into()))?;
    if source.len() > EDITOR_MAX_FILE_BYTES {
        return Err((413, "source exceeds the 1 MiB editor limit".into()));
    }
    let cursor = request
        .get("offset")
        .and_then(Value::as_u64)
        .ok_or((400, "cursor position is required".into()))?;
    let cursor_byte = utf16_to_byte(source, usize::try_from(cursor).unwrap_or(usize::MAX));
    let contexts = call_contexts(source, cursor_byte);
    if contexts.is_empty() {
        return Ok(not_found(request));
    }

    let mut overlays = editor_analysis_overlays(state, request)?;
    let current = overlays
        .iter_mut()
        .find(|overlay| overlay.path == relative)
        .ok_or((400, "source overlay is missing".into()))?;
    current
        .text
        .insert_str(cursor_byte, "__zelyra_signature_hole");

    let entry = state.root.join("main.zyl");
    if !entry.is_file() {
        return Err((422, "project entry file main.zyl is missing".into()));
    }
    let analysis = language_service::analyze(&entry, &overlays);
    let Some(program) = analysis.program.as_ref() else {
        return Ok(not_found(request));
    };
    let index = semantic_index::SemanticIndex::from_analysis(&analysis);
    for context in contexts {
        let Some(occurrence) = index
            .at(relative, context.callee_start)
            .filter(|occurrence| !occurrence.declaration)
        else {
            continue;
        };
        let Some(symbol) = index.symbol(&occurrence.key) else {
            continue;
        };
        let Some(function) = program
            .functions
            .iter()
            .find(|function| function.name == symbol.key)
        else {
            continue;
        };
        if function.params.is_empty() {
            continue;
        }
        let source_id = analysis
            .sources
            .iter()
            .position(|item| item.path == relative)
            .unwrap_or(0) as u32;
        if function.span.source_id != source_id && !function.is_public {
            continue;
        }
        let name = symbol.name.clone();
        let parameters = function
            .params
            .iter()
            .map(|parameter| json!({"name":parameter.name,"type":parameter.ty.to_string()}))
            .collect::<Vec<_>>();
        let response = json!({
            "found": true,
            "name": name,
            "typeParameters": function.type_params,
            "parameters": parameters,
            "returnType": function.return_type.as_ref().map_or_else(|| "Unit".to_owned(), ToString::to_string),
            "activeParameter": context.active_parameter,
            "version": request.get("version"),
        });
        return Ok((200, "application/json; charset=utf-8", response.to_string()));
    }
    Ok(not_found(request))
}

fn not_found(request: &Value) -> (u16, &'static str, String) {
    (
        200,
        "application/json; charset=utf-8",
        json!({"found":false,"version":request.get("version")}).to_string(),
    )
}

fn call_contexts(source: &str, cursor_byte: usize) -> Vec<CallContext> {
    let Ok(tokens) = lex(source) else {
        return Vec::new();
    };
    if cursor_byte > source.len()
        || !source.is_char_boundary(cursor_byte)
        || tokens.iter().any(|token| {
            token.span.start < cursor_byte
                && cursor_byte < token.span.end
                && matches!(
                    &token.kind,
                    TokenKind::String(_)
                        | TokenKind::StringTemplate { .. }
                        | TokenKind::Char(_)
                        | TokenKind::SqlBody(_)
                        | TokenKind::HtmlBody(_)
                )
        })
    {
        return Vec::new();
    }

    let mut delimiters = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.span.start >= cursor_byte {
            break;
        }
        match &token.kind {
            TokenKind::LParen => {
                let is_call = function_name_index_before_open(&tokens, index).is_some();
                delimiters.push(if is_call {
                    Delimiter::Call {
                        open_index: index,
                        commas: 0,
                    }
                } else {
                    Delimiter::Other('(')
                });
            }
            TokenKind::LBracket => delimiters.push(Delimiter::Other('[')),
            TokenKind::LBrace => delimiters.push(Delimiter::Other('{')),
            TokenKind::RParen => close_delimiter(&mut delimiters, '('),
            TokenKind::RBracket => close_delimiter(&mut delimiters, '['),
            TokenKind::RBrace => close_delimiter(&mut delimiters, '{'),
            TokenKind::Comma => {
                if let Some(Delimiter::Call { commas, .. }) = delimiters.last_mut() {
                    *commas += 1;
                }
            }
            _ => {}
        }
    }

    let mut contexts = Vec::new();
    for delimiter in delimiters.iter().rev() {
        if let Delimiter::Call { open_index, commas } = delimiter {
            if let Some(context) = call_context_for_open(source, &tokens, *open_index, *commas) {
                contexts.push(context);
            }
        }
    }
    contexts
}

fn close_delimiter(delimiters: &mut Vec<Delimiter>, closing: char) {
    if let Some(index) = delimiters.iter().rposition(|delimiter| match delimiter {
        Delimiter::Call { .. } => closing == '(',
        Delimiter::Other(opening) => {
            matches!((*opening, closing), ('(', ')') | ('[', ']') | ('{', '}'))
        }
    }) {
        delimiters.truncate(index);
    }
}

fn call_context_for_open(
    source: &str,
    tokens: &[Token],
    open_index: usize,
    commas: usize,
) -> Option<CallContext> {
    let name_index = function_name_index_before_open(tokens, open_index)?;
    let mut callee_start_index = name_index;
    while callee_start_index >= 2
        && tokens[callee_start_index - 1].kind == TokenKind::DoubleColon
        && matches!(tokens[callee_start_index - 2].kind, TokenKind::Ident(_))
    {
        callee_start_index -= 2;
    }
    let start = tokens.get(callee_start_index)?.span.start;
    let end = tokens.get(name_index)?.span.end;
    source.get(start..end)?;
    Some(CallContext {
        callee_start: tokens.get(name_index)?.span.start,
        active_parameter: commas,
    })
}

fn function_name_index_before_open(tokens: &[Token], open_index: usize) -> Option<usize> {
    let mut name_index = open_index.checked_sub(1)?;
    if matches!(&tokens.get(name_index)?.kind, TokenKind::Greater) {
        let mut depth = 1usize;
        name_index = name_index.checked_sub(1)?;
        while depth > 0 {
            match &tokens.get(name_index)?.kind {
                TokenKind::Greater => depth += 1,
                TokenKind::Less => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                name_index = name_index.checked_sub(1)?;
            }
        }
        name_index = name_index.checked_sub(1)?;
    }
    matches!(&tokens.get(name_index)?.kind, TokenKind::Ident(_)).then_some(name_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_context_counts_only_top_level_arguments_and_ignores_literals() {
        let source = "fn main() { outer(1, nested([2, 3], \"comma, and (paren)\"), ) }";
        let cursor = source.find("), )").unwrap() + 3;
        let contexts = call_contexts(source, cursor);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].active_parameter, 2);
        assert_eq!(
            &source[contexts[0].callee_start..contexts[0].callee_start + "outer".len()],
            "outer"
        );
    }

    #[test]
    fn call_context_is_suppressed_inside_strings() {
        let source = "fn main() { print(\"value with (not a call\") }";
        let cursor = source.find("not a call").unwrap() + 4;
        assert!(call_contexts(source, cursor).is_empty());
    }

    #[test]
    fn call_context_recognizes_explicit_nested_generic_arguments() {
        let source = "fn main() { find<Map<String, Int>>(1, ) }";
        let cursor = source.find(", )").unwrap() + 2;
        let contexts = call_contexts(source, cursor);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].active_parameter, 1);
    }
}
