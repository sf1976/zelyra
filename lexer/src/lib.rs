use zelyra_ast::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Fn,
    Return,
    If,
    Else,
    While,
    Loop,
    Break,
    Mutable,
    True,
    False,
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Arrow,
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
            let kind = match word {
                "fn" => TokenKind::Fn,
                "return" => TokenKind::Return,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "loop" => TokenKind::Loop,
                "break" => TokenKind::Break,
                "mutable" => TokenKind::Mutable,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                _ => TokenKind::Ident(word.to_owned()),
            };
            tokens.push(Token {
                kind,
                span: span(i),
            });
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
        let (kind, width) = match (c, bytes.get(i + 1).copied().map(char::from)) {
            ('-', Some('>')) => (TokenKind::Arrow, 2),
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
            kind,
            span: span(i),
        });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(source.len(), source.len(), line, column),
    });
    Ok(tokens)
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
}
