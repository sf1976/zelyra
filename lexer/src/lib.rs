use zelyra_ast::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Fn,
    Type,
    Database,
    Table,
    Engine,
    Postgres,
    Primary,
    Auto,
    Required,
    Default,
    Index,
    Unique,
    Return,
    If,
    Else,
    While,
    Invariant,
    Loop,
    Break,
    Continue,
    Mutable,
    Match,
    Sql,
    Page,
    Html,
    Form,
    Crud,
    Auth,
    Requires,
    Permits,
    Sessions,
    Permissions,
    Title,
    List,
    Search,
    Filter,
    Field,
    Action,
    Label,
    Placeholder,
    Max,
    Widget,
    Readonly,
    Fields,
    Success,
    Redirect,
    Transaction,
    Uses,
    Ensures,
    True,
    False,
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Arrow,
    FatArrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AndAnd,
    OrOr,
    DotDot,
    Colon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Question,
    LBracket,
    RBracket,
    SqlBody(String),
    HtmlBody(String),
    Newline,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut column = 1;
    let mut sql_pending = false;
    let mut sql_header_ready = false;
    let mut html_pending = false;

    while i < bytes.len() {
        let start = i;
        let token_line = line;
        let token_column = column;
        let c = bytes[i] as char;
        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            column += 1;
            continue;
        }
        if c == '\n' {
            tokens.push(Token {
                kind: TokenKind::Newline,
                span: Span::new(i, i + 1, line, column),
            });
            i += 1;
            line += 1;
            column = 1;
            continue;
        }
        if c == '/' && bytes.get(i + 1).copied() == Some(b'/') {
            i += 2;
            column += 2;
            while i < bytes.len() && bytes[i] as char != '\n' {
                i += 1;
                column += 1;
            }
            continue;
        }
        let span = |end: usize| Span::new(start, end, token_line, token_column);
        if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            column += 1;
            while i < bytes.len()
                && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] as char == '_')
            {
                i += 1;
                column += 1;
            }
            let word = &source[start..i];
            if sql_pending && !sql_header_ready && word != "sql" {
                sql_pending = false;
            }
            if html_pending && word != "html" {
                html_pending = false;
            }
            let kind = match word {
                "fn" => TokenKind::Fn,
                "type" => TokenKind::Type,
                "database" => TokenKind::Database,
                "table" => TokenKind::Table,
                "engine" => TokenKind::Engine,
                "postgres" => TokenKind::Postgres,
                "page" => TokenKind::Page,
                "html" => TokenKind::Html,
                "form" => TokenKind::Form,
                "crud" => TokenKind::Crud,
                "auth" => TokenKind::Auth,
                "requires" => TokenKind::Requires,
                "permits" => TokenKind::Permits,
                "sessions" => TokenKind::Sessions,
                "permissions" => TokenKind::Permissions,
                "title" => TokenKind::Title,
                "list" => TokenKind::List,
                "search" => TokenKind::Search,
                "filter" => TokenKind::Filter,
                "field" => TokenKind::Field,
                "action" => TokenKind::Action,
                "label" => TokenKind::Label,
                "placeholder" => TokenKind::Placeholder,
                "max" => TokenKind::Max,
                "widget" => TokenKind::Widget,
                "readonly" => TokenKind::Readonly,
                "fields" => TokenKind::Fields,
                "success" => TokenKind::Success,
                "redirect" => TokenKind::Redirect,
                "primary" => TokenKind::Primary,
                "auto" => TokenKind::Auto,
                "required" => TokenKind::Required,
                "default" => TokenKind::Default,
                "index" => TokenKind::Index,
                "unique" => TokenKind::Unique,
                "return" => TokenKind::Return,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "invariant" => TokenKind::Invariant,
                "loop" => TokenKind::Loop,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "mutable" => TokenKind::Mutable,
                "match" => TokenKind::Match,
                "sql" => TokenKind::Sql,
                "transaction" => TokenKind::Transaction,
                "uses" => TokenKind::Uses,
                "ensures" => TokenKind::Ensures,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                _ => TokenKind::Ident(word.to_owned()),
            };
            tokens.push(Token {
                kind,
                span: span(i),
            });
            if word == "sql" {
                sql_pending = true;
            }
            if word == "html" {
                html_pending = true;
            }
            continue;
        }
        if c.is_ascii_digit() {
            i += 1;
            column += 1;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
                column += 1;
            }
            let mut is_float = false;
            if bytes.get(i) == Some(&b'.')
                && bytes
                    .get(i + 1)
                    .is_some_and(|b| (*b as char).is_ascii_digit())
            {
                is_float = true;
                i += 1;
                column += 1;
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                    column += 1;
                }
            }
            let raw = &source[start..i];
            let kind = if is_float {
                TokenKind::Float(raw.parse().map_err(|_| LexError {
                    message: format!("invalid float literal `{raw}`"),
                    span: span(i),
                })?)
            } else {
                TokenKind::Int(raw.parse().map_err(|_| LexError {
                    message: format!("invalid integer literal `{raw}`"),
                    span: span(i),
                })?)
            };
            tokens.push(Token {
                kind,
                span: span(i),
            });
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            column += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch == quote {
                    i += 1;
                    column += 1;
                    closed = true;
                    break;
                }
                if ch == '\n' {
                    return Err(LexError {
                        message: "unterminated literal".into(),
                        span: span(i),
                    });
                }
                if ch == '\\' {
                    i += 1;
                    column += 1;
                    let escaped =
                        bytes
                            .get(i)
                            .copied()
                            .map(char::from)
                            .ok_or_else(|| LexError {
                                message: "unterminated escape".into(),
                                span: span(i),
                            })?;
                    let decoded = match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        '\'' => '\'',
                        _ => {
                            return Err(LexError {
                                message: format!("unknown escape `\\{escaped}`"),
                                span: span(i),
                            })
                        }
                    };
                    value.push(decoded);
                    i += 1;
                    column += 1;
                } else {
                    value.push(ch);
                    i += 1;
                    column += 1;
                }
            }
            if !closed {
                return Err(LexError {
                    message: "unterminated literal".into(),
                    span: span(i),
                });
            }
            let kind = if quote == '"' {
                TokenKind::String(value)
            } else {
                let mut chars = value.chars();
                let ch = chars.next().ok_or_else(|| LexError {
                    message: "empty character literal".into(),
                    span: span(i),
                })?;
                if chars.next().is_some() {
                    return Err(LexError {
                        message: "character literal must contain exactly one character".into(),
                        span: span(i),
                    });
                }
                TokenKind::Char(ch)
            };
            tokens.push(Token {
                kind,
                span: span(i),
            });
            continue;
        }
        if c == '{' && (sql_header_ready || sql_pending || html_pending) {
            let open_span = span(i + 1);
            tokens.push(Token {
                kind: TokenKind::LBrace,
                span: open_span,
            });
            i += 1;
            column += 1;
            let body_start = i;
            let (body_end, next_line, next_column) =
                scan_raw_body(source, body_start, line, column)?;
            let body = source[body_start..body_end].to_owned();
            let body_kind = if html_pending {
                TokenKind::HtmlBody(body)
            } else {
                TokenKind::SqlBody(body)
            };
            tokens.push(Token {
                kind: body_kind,
                span: Span::new(body_start, body_end, line, column),
            });
            let close_span = Span::new(body_end, body_end + 1, next_line, next_column);
            tokens.push(Token {
                kind: TokenKind::RBrace,
                span: close_span,
            });
            i = body_end + 1;
            line = next_line;
            column = next_column + 1;
            sql_pending = false;
            sql_header_ready = false;
            html_pending = false;
            continue;
        }
        let (kind, width) = match (c, bytes.get(i + 1).copied().map(char::from)) {
            ('-', Some('>')) => (TokenKind::Arrow, 2),
            ('=', Some('>')) => (TokenKind::FatArrow, 2),
            ('=', Some('=')) => (TokenKind::EqualEqual, 2),
            ('!', Some('=')) => (TokenKind::NotEqual, 2),
            ('<', Some('=')) => (TokenKind::LessEqual, 2),
            ('>', Some('=')) => (TokenKind::GreaterEqual, 2),
            ('&', Some('&')) => (TokenKind::AndAnd, 2),
            ('|', Some('|')) => (TokenKind::OrOr, 2),
            ('.', Some('.')) => (TokenKind::DotDot, 2),
            ('+', _) => (TokenKind::Plus, 1),
            ('-', _) => (TokenKind::Minus, 1),
            ('*', _) => (TokenKind::Star, 1),
            ('/', _) => (TokenKind::Slash, 1),
            ('%', _) => (TokenKind::Percent, 1),
            ('!', _) => (TokenKind::Bang, 1),
            ('=', _) => (TokenKind::Equal, 1),
            ('<', _) => (TokenKind::Less, 1),
            ('>', _) => (TokenKind::Greater, 1),
            (':', _) => (TokenKind::Colon, 1),
            (',', _) => (TokenKind::Comma, 1),
            ('(', _) => (TokenKind::LParen, 1),
            (')', _) => (TokenKind::RParen, 1),
            ('{', _) => (TokenKind::LBrace, 1),
            ('}', _) => (TokenKind::RBrace, 1),
            (';', _) => (TokenKind::Semicolon, 1),
            ('?', _) => (TokenKind::Question, 1),
            ('[', _) => (TokenKind::LBracket, 1),
            (']', _) => (TokenKind::RBracket, 1),
            _ => {
                return Err(LexError {
                    message: format!("unexpected character `{c}`"),
                    span: span(i + 1),
                })
            }
        };
        i += width;
        column += width;
        tokens.push(Token {
            kind: kind.clone(),
            span: span(i),
        });
        if sql_pending {
            match kind {
                TokenKind::Less => sql_header_ready = true,
                TokenKind::LParen | TokenKind::Equal => {
                    sql_pending = false;
                    sql_header_ready = false;
                }
                _ => {}
            }
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(source.len(), source.len(), line, column),
    });
    Ok(tokens)
}

fn scan_raw_body(
    source: &str,
    start: usize,
    mut line: usize,
    mut column: usize,
) -> Result<(usize, usize, usize), LexError> {
    let bytes = source.as_bytes();
    let mut i = start;
    let mut quote = None;
    let mut brace_depth = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if let Some(active_quote) = quote {
            if c == active_quote {
                if bytes.get(i + 1).copied() == Some(active_quote as u8) {
                    i += 2;
                    column += 2;
                    continue;
                }
                quote = None;
                i += 1;
                column += 1;
                continue;
            }
        } else if c == '\'' || c == '"' {
            quote = Some(c);
            i += 1;
            column += 1;
            continue;
        } else if c == '{' {
            brace_depth += 1;
        } else if c == '}' {
            if brace_depth == 0 {
                return Ok((i, line, column));
            }
            brace_depth -= 1;
        }
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
        i += 1;
    }
    Err(LexError {
        message: "unterminated raw block".into(),
        span: Span::new(start, source.len(), line, column),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexes_keywords_and_operators() {
        let tokens = lex("fn main() { return 1 + 2 >= 3 }").unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Fn));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::GreaterEqual));
    }
    #[test]
    fn lexes_escaped_strings() {
        assert_eq!(
            lex(r#""hello\n""#).unwrap()[0].kind,
            TokenKind::String("hello\n".into())
        );
    }

    #[test]
    fn captures_native_sql_without_lexing_sql_literals() {
        let tokens = lex("fn main() { rows = sql<Customer[]> { SELECT name FROM customers WHERE name = 'Anna' } }").unwrap();
        assert!(tokens.iter().any(|token| matches!(
            &token.kind,
            TokenKind::SqlBody(body) if body.contains("'Anna'")
        )));
    }

    #[test]
    fn captures_html_body_and_template_braces() {
        let tokens = lex("page \"/hello/{name}\" { html { <h1>Hello, {name}!</h1> } }").unwrap();
        assert!(tokens.iter().any(|token| matches!(
            &token.kind,
            TokenKind::HtmlBody(body) if body.contains("{name}")
        )));
    }
}
