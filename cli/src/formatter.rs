use std::fmt::Write as _;

use zelyra_lexer::{Token, TokenKind};

/// Format a syntactically valid Zelyra source file.
///
/// Formatting is intentionally token based for now.  This keeps raw SQL and
/// HTML bodies opaque while making the language surface deterministic.  The
/// parser is run by the CLI before this function is called, so the formatter
/// never rewrites an invalid program.
pub fn format_source(source: &str, tokens: &[Token]) -> String {
    Formatter::new(source, tokens).format()
}

struct Formatter<'a> {
    source: &'a str,
    tokens: &'a [Token],
    output: String,
    index: usize,
    previous_end: usize,
    indent: usize,
    paren_depth: usize,
    bracket_depth: usize,
    generic_depth: usize,
    pending_block_break: bool,
    previous_kind: Option<TokenKind>,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            output: String::new(),
            index: 0,
            previous_end: 0,
            indent: 0,
            paren_depth: 0,
            bracket_depth: 0,
            generic_depth: 0,
            pending_block_break: false,
            previous_kind: None,
        }
    }

    fn format(mut self) -> String {
        while self.index < self.tokens.len() {
            let token = &self.tokens[self.index];
            self.emit_comments(self.previous_end, token.span.start);

            match &token.kind {
                TokenKind::Eof => break,
                TokenKind::Newline | TokenKind::Semicolon => self.emit_newline(),
                TokenKind::LBrace => self.left_brace(),
                TokenKind::RBrace => self.right_brace(),
                TokenKind::LParen => self.left_paren(),
                TokenKind::RParen => self.right_paren(),
                TokenKind::LBracket => self.left_bracket(),
                TokenKind::RBracket => self.right_bracket(),
                TokenKind::Comma => self.comma(),
                TokenKind::Colon => self.colon(),
                TokenKind::Dot => self.punctuation("."),
                TokenKind::Question => self.punctuation("?"),
                TokenKind::Arrow
                | TokenKind::FatArrow
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Equal
                | TokenKind::EqualEqual
                | TokenKind::NotEqual
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::AndAnd
                | TokenKind::OrOr => self.operator(token),
                TokenKind::Bang => self.prefix_operator("!"),
                _ => self.word_or_body(token),
            }

            self.previous_end = token.span.end;
            self.previous_kind = Some(token.kind.clone());
            self.index += 1;
        }

        self.emit_comments(self.previous_end, self.source.len());
        self.output = self.output.trim_end().to_owned();
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output
    }

    fn emit_comments(&mut self, start: usize, end: usize) {
        if start >= end || end > self.source.len() {
            return;
        }
        let gap = &self.source[start..end];
        let Some(comment_start) = gap.find("//") else {
            return;
        };
        let comment = gap[comment_start..].trim_end_matches(['\r', '\n', ' ', '\t']);
        if comment.is_empty() {
            return;
        }
        if !self.at_line_start() {
            self.output.push(' ');
        }
        self.push_text(comment);
    }

    fn left_brace(&mut self) {
        self.ensure_pending_block_break();
        self.space_before();
        self.push_text("{");
        self.indent += 1;
        self.pending_block_break = true;
    }

    fn right_brace(&mut self) {
        self.ensure_pending_block_break();
        self.indent = self.indent.saturating_sub(1);
        self.ensure_line_start();
        self.push_text("}");
        self.pending_block_break = false;
        if !self.next_significant_is_else() {
            self.emit_newline();
        }
    }

    fn left_paren(&mut self) {
        self.ensure_pending_block_break();
        self.push_text("(");
        self.paren_depth += 1;
    }

    fn right_paren(&mut self) {
        self.trim_spaces();
        self.push_text(")");
        self.paren_depth = self.paren_depth.saturating_sub(1);
    }

    fn left_bracket(&mut self) {
        self.ensure_pending_block_break();
        self.push_text("[");
        self.bracket_depth += 1;
    }

    fn right_bracket(&mut self) {
        self.trim_spaces();
        self.push_text("]");
        self.bracket_depth = self.bracket_depth.saturating_sub(1);
    }

    fn comma(&mut self) {
        self.trim_spaces();
        self.push_text(",");
        self.push_text(" ");
    }

    fn colon(&mut self) {
        self.trim_spaces();
        self.push_text(":");
        self.push_text(" ");
    }

    fn punctuation(&mut self, text: &str) {
        self.trim_spaces();
        self.push_text(text);
    }

    fn operator(&mut self, token: &Token) {
        let text = token_text(token, self.source);
        if matches!(token.kind, TokenKind::Less) && self.is_generic_open() {
            self.trim_spaces();
            self.push_text("<");
            self.generic_depth += 1;
            return;
        }
        if matches!(token.kind, TokenKind::Greater) && self.generic_depth > 0 {
            self.trim_spaces();
            self.push_text(">");
            self.generic_depth -= 1;
            return;
        }
        if matches!(token.kind, TokenKind::Minus) && self.is_prefix_position() {
            self.prefix_operator(text);
            return;
        }
        self.trim_spaces();
        self.space_before();
        self.push_text(text);
        self.push_text(" ");
    }

    fn prefix_operator(&mut self, text: &str) {
        self.trim_spaces();
        self.push_text(text);
    }

    fn word_or_body(&mut self, token: &Token) {
        self.ensure_pending_block_break();
        if matches!(token.kind, TokenKind::SqlBody(_) | TokenKind::HtmlBody(_)) {
            self.ensure_line_start();
            let body = token_text(token, self.source);
            let body = body.trim_matches(['\r', '\n', ' ', '\t']);
            self.push_text(body);
            return;
        }
        if self.needs_word_space() {
            self.space_before();
        }
        self.push_text(token_text(token, self.source));
    }

    fn is_prefix_position(&self) -> bool {
        matches!(
            self.previous_kind,
            None | Some(
                TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
                    | TokenKind::Comma
                    | TokenKind::Colon
                    | TokenKind::Equal
                    | TokenKind::Arrow
                    | TokenKind::FatArrow
                    | TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::EqualEqual
                    | TokenKind::NotEqual
                    | TokenKind::Less
                    | TokenKind::LessEqual
                    | TokenKind::Greater
                    | TokenKind::GreaterEqual
                    | TokenKind::AndAnd
                    | TokenKind::OrOr
                    | TokenKind::Bang
            )
        )
    }

    fn needs_word_space(&self) -> bool {
        self.previous_kind
            .as_ref()
            .is_some_and(|kind| !is_compact_token(kind))
    }

    fn ensure_pending_block_break(&mut self) {
        if self.pending_block_break {
            self.ensure_line_start();
            self.pending_block_break = false;
        }
    }

    fn next_significant_is_else(&self) -> bool {
        self.tokens
            .iter()
            .skip(self.index + 1)
            .find(|token| !matches!(token.kind, TokenKind::Newline | TokenKind::Semicolon))
            .is_some_and(|token| matches!(token.kind, TokenKind::Else))
    }

    fn is_generic_open(&self) -> bool {
        if !self
            .previous_kind
            .as_ref()
            .is_some_and(|kind| !is_compact_token(kind))
        {
            return false;
        }
        self.tokens
            .iter()
            .skip(self.index + 1)
            .take_while(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Newline | TokenKind::LBrace | TokenKind::RBrace | TokenKind::Equal
                )
            })
            .any(|token| matches!(token.kind, TokenKind::Greater))
    }

    fn emit_newline(&mut self) {
        self.pending_block_break = false;
        if self.bracket_depth > 0 {
            return;
        }
        self.trim_spaces();
        if !self.at_line_start() {
            self.output.push('\n');
        }
    }

    fn ensure_line_start(&mut self) {
        self.trim_spaces();
        if !self.at_line_start() {
            self.output.push('\n');
        }
    }

    fn space_before(&mut self) {
        if !self.at_line_start() && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
    }

    fn trim_spaces(&mut self) {
        while self.output.ends_with(' ') || self.output.ends_with('\t') {
            self.output.pop();
        }
    }

    fn at_line_start(&self) -> bool {
        self.output.is_empty() || self.output.ends_with('\n')
    }

    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.at_line_start() {
            let _ = write!(self.output, "{}", "    ".repeat(self.indent));
        }
        self.output.push_str(text);
    }
}

fn is_compact_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline
            | TokenKind::Eof
            | TokenKind::Semicolon
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::Question
            | TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Bang
            | TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::NotEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::AndAnd
            | TokenKind::OrOr
    )
}

fn token_text<'a>(token: &'a Token, source: &'a str) -> &'a str {
    match &token.kind {
        TokenKind::Ident(_)
        | TokenKind::Int(_)
        | TokenKind::Float(_)
        | TokenKind::String(_)
        | TokenKind::Char(_) => source
            .get(token.span.start..token.span.end)
            .unwrap_or_default(),
        TokenKind::SqlBody(body) | TokenKind::HtmlBody(body) => body,
        TokenKind::Fn => "fn",
        TokenKind::Type => "type",
        TokenKind::Struct => "struct",
        TokenKind::Database => "database",
        TokenKind::Table => "table",
        TokenKind::Engine => "engine",
        TokenKind::Postgres => "postgres",
        TokenKind::Primary => "primary",
        TokenKind::Auto => "auto",
        TokenKind::Required => "required",
        TokenKind::Default => "default",
        TokenKind::Index => "index",
        TokenKind::Unique => "unique",
        TokenKind::Return => "return",
        TokenKind::If => "if",
        TokenKind::Else => "else",
        TokenKind::For => "for",
        TokenKind::In => "in",
        TokenKind::While => "while",
        TokenKind::Invariant => "invariant",
        TokenKind::Loop => "loop",
        TokenKind::Break => "break",
        TokenKind::Continue => "continue",
        TokenKind::Mutable => "mutable",
        TokenKind::Match => "match",
        TokenKind::Sql => "sql",
        TokenKind::Page => "page",
        TokenKind::TableView => "tableview",
        TokenKind::View => "view",
        TokenKind::Component => "component",
        TokenKind::Props => "props",
        TokenKind::Html => "html",
        TokenKind::Form => "form",
        TokenKind::Crud => "crud",
        TokenKind::Auth => "auth",
        TokenKind::Api => "api",
        TokenKind::Input => "input",
        TokenKind::Output => "output",
        TokenKind::Errors => "errors",
        TokenKind::Handler => "handler",
        TokenKind::Requires => "requires",
        TokenKind::Permits => "permits",
        TokenKind::Sessions => "sessions",
        TokenKind::Permissions => "permissions",
        TokenKind::Roles => "roles",
        TokenKind::RolePermissions => "role_permissions",
        TokenKind::Audit => "audit",
        TokenKind::AuditChain => "audit_chain",
        TokenKind::AdminPath => "admin_path",
        TokenKind::AdminPermission => "admin_permission",
        TokenKind::AdminRole => "admin_role",
        TokenKind::Title => "title",
        TokenKind::Layout => "layout",
        TokenKind::List => "list",
        TokenKind::Detail => "detail",
        TokenKind::Search => "search",
        TokenKind::Sort => "sort",
        TokenKind::Sortable => "sortable",
        TokenKind::Searchable => "searchable",
        TokenKind::Paginated => "paginated",
        TokenKind::Source => "source",
        TokenKind::Columns => "columns",
        TokenKind::Filter => "filter",
        TokenKind::SoftDelete => "soft_delete",
        TokenKind::Field => "field",
        TokenKind::Action => "action",
        TokenKind::Label => "label",
        TokenKind::Icon => "icon",
        TokenKind::Placeholder => "placeholder",
        TokenKind::Max => "max",
        TokenKind::Widget => "widget",
        TokenKind::Readonly => "readonly",
        TokenKind::Fields => "fields",
        TokenKind::Success => "success",
        TokenKind::Redirect => "redirect",
        TokenKind::Confirm => "confirm",
        TokenKind::ConfirmPage => "confirm_page",
        TokenKind::SuccessPage => "success_page",
        TokenKind::ErrorPage => "error_page",
        TokenKind::Transaction => "transaction",
        TokenKind::Parallel => "parallel",
        TokenKind::Await => "await",
        TokenKind::Uses => "uses",
        TokenKind::Ensures => "ensures",
        TokenKind::True => "true",
        TokenKind::False => "false",
        TokenKind::Arrow => "->",
        TokenKind::FatArrow => "=>",
        TokenKind::Plus => "+",
        TokenKind::Minus => "-",
        TokenKind::Star => "*",
        TokenKind::Slash => "/",
        TokenKind::Percent => "%",
        TokenKind::Bang => "!",
        TokenKind::Equal => "=",
        TokenKind::EqualEqual => "==",
        TokenKind::NotEqual => "!=",
        TokenKind::Less => "<",
        TokenKind::LessEqual => "<=",
        TokenKind::Greater => ">",
        TokenKind::GreaterEqual => ">=",
        TokenKind::AndAnd => "&&",
        TokenKind::OrOr => "||",
        TokenKind::Dot => ".",
        TokenKind::DotDot => "..",
        TokenKind::Colon => ":",
        TokenKind::Comma => ",",
        TokenKind::LParen => "(",
        TokenKind::RParen => ")",
        TokenKind::LBrace => "{",
        TokenKind::RBrace => "}",
        TokenKind::Semicolon => ";",
        TokenKind::Question => "?",
        TokenKind::LBracket => "[",
        TokenKind::RBracket => "]",
        TokenKind::Newline | TokenKind::Eof => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    fn format(source: &str) -> String {
        let tokens = lex(source).expect("source should lex");
        parse(&tokens).expect("source should parse");
        format_source(source, &tokens)
    }

    #[test]
    fn formats_blocks_and_is_idempotent() {
        let source = "fn main(){mutable total=1+2\nprint(total)}";
        let formatted = format(source);
        assert_eq!(
            formatted,
            "fn main() {\n    mutable total = 1 + 2\n    print(total)\n}\n"
        );
        assert_eq!(format(&formatted), formatted);
    }

    #[test]
    fn preserves_comments_and_raw_bodies() {
        let source = "// keep this\npage \"/\"{html{<h1>Hello</h1>}}";
        let formatted = format(source);
        assert!(formatted.contains("// keep this"));
        assert!(
            formatted.contains("html {\n        <h1>Hello</h1>\n    }"),
            "formatted: {formatted:?}"
        );
        let tokens = lex(&formatted).expect("formatted source should lex");
        parse(&tokens).expect("formatted source should parse");
    }

    #[test]
    fn keeps_multiline_sql_parseable() {
        let source = "fn load() uses Database { value=sql<String>{SELECT id FROM users WHERE id=:id} print(value) }";
        let formatted = format(source);
        assert!(
            formatted.contains("sql<String> {\n        SELECT id FROM users WHERE id=:id\n    }"),
            "formatted: {formatted:?}"
        );
        let tokens = lex(&formatted).expect("formatted source should lex");
        parse(&tokens).expect("formatted source should parse");
    }
}
