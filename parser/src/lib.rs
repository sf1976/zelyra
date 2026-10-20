use std::collections::HashSet;
use std::fmt;
use zelyra_ast::*;
use zelyra_lexer::{lex, Token, TokenKind};

type ParsedColumnType = (Type, Option<u32>, Option<(u32, u32)>);

const MAX_NESTING_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut complete_tokens = tokens.to_vec();
    if !matches!(
        complete_tokens.last().map(|token| &token.kind),
        Some(TokenKind::Eof)
    ) {
        let span = complete_tokens
            .last()
            .map_or_else(Span::default, |token| token.span);
        complete_tokens.push(Token {
            kind: TokenKind::Eof,
            span,
        });
    }
    Parser {
        tokens: &complete_tokens,
        pos: 0,
        nesting_depth: 0,
    }
    .program()
}

fn crud_record_action(scope: Option<&str>) -> Option<zelyra_ast::CrudRecordAction> {
    match scope {
        Some("edit") => Some(zelyra_ast::CrudRecordAction::Edit),
        Some("delete") => Some(zelyra_ast::CrudRecordAction::Delete),
        _ => None,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    nesting_depth: usize,
}

impl<'a> Parser<'a> {
    fn with_nesting<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        if self.nesting_depth >= MAX_NESTING_DEPTH {
            return self.error(format!(
                "maximum parser nesting depth of {MAX_NESTING_DEPTH} exceeded"
            ));
        }
        self.nesting_depth += 1;
        let result = parse(self);
        self.nesting_depth -= 1;
        result
    }

    fn current(&self) -> &'a Token {
        &self.tokens[self.pos]
    }
    fn at(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }
    fn at_word(&self, word: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Ident(name) if name == word)
    }
    fn at_word_followed_by(&self, word: &str, kind: &TokenKind) -> bool {
        self.at_word(word)
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| &token.kind == kind)
    }
    fn advance(&mut self) -> &'a Token {
        let token = self.current();
        if !matches!(&token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }
    fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) || self.at(&TokenKind::Semicolon) {
            self.pos += 1;
        }
    }
    fn error<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            message: message.into(),
            span: self.current().span,
        })
    }
    fn expect(&mut self, kind: TokenKind, label: &str) -> Result<Span, ParseError> {
        if self.at(&kind) {
            Ok(self.advance().span)
        } else {
            self.error(format!("expected {label}, found {:?}", self.current().kind))
        }
    }
    fn ident(&mut self, label: &str) -> Result<(String, Span), ParseError> {
        match &self.current().kind {
            // These contextual words are useful as finite UI enum variants
            // (for example `enum UiAppearance { default success warning }`)
            // while remaining keywords everywhere else in the grammar.
            TokenKind::Default => {
                let span = self.advance().span;
                Ok(("default".into(), span))
            }
            TokenKind::Success => {
                let span = self.advance().span;
                Ok(("success".into(), span))
            }
            TokenKind::Max => {
                let span = self.advance().span;
                Ok(("max".into(), span))
            }
            TokenKind::Unique => {
                let span = self.advance().span;
                Ok(("unique".into(), span))
            }
            TokenKind::Sort => {
                let span = self.advance().span;
                Ok(("sort".into(), span))
            }
            TokenKind::Columns => {
                let span = self.advance().span;
                Ok(("columns".into(), span))
            }
            TokenKind::Appearance => {
                let span = self.advance().span;
                Ok(("appearance".into(), span))
            }
            TokenKind::Placeholder => {
                let span = self.advance().span;
                Ok(("placeholder".into(), span))
            }
            TokenKind::Readonly => {
                let span = self.advance().span;
                Ok(("readonly".into(), span))
            }
            TokenKind::Required => {
                let span = self.advance().span;
                Ok(("required".into(), span))
            }
            TokenKind::Action => {
                let span = self.advance().span;
                Ok(("action".into(), span))
            }
            TokenKind::Ident(name) => {
                let span = self.advance().span;
                Ok((name.clone(), span))
            }
            TokenKind::Roles => {
                let span = self.advance().span;
                Ok(("roles".into(), span))
            }
            TokenKind::RolePermissions => {
                let span = self.advance().span;
                Ok(("role_permissions".into(), span))
            }
            TokenKind::Audit => {
                let span = self.advance().span;
                Ok(("audit".into(), span))
            }
            TokenKind::View => {
                let span = self.advance().span;
                Ok(("view".into(), span))
            }
            TokenKind::Label => {
                let span = self.advance().span;
                Ok(("label".into(), span))
            }
            TokenKind::Title => {
                let span = self.advance().span;
                Ok(("title".into(), span))
            }
            _ => self.error(format!("expected {label}")),
        }
    }

    /// Field names are parsed in a context that can safely admit selected
    /// declaration keywords without treating them as general identifiers.
    fn field_ident(&mut self, label: &str) -> Result<(String, Span), ParseError> {
        match &self.current().kind {
            TokenKind::Type => {
                let span = self.advance().span;
                Ok(("type".into(), span))
            }
            TokenKind::Version => {
                let span = self.advance().span;
                Ok(("version".into(), span))
            }
            _ => self.ident(label),
        }
    }

    fn table_ident(&mut self, label: &str) -> Result<(String, Span), ParseError> {
        match &self.current().kind {
            TokenKind::Sessions => {
                let span = self.advance().span;
                Ok(("sessions".into(), span))
            }
            _ => self.ident(label),
        }
    }
    fn program(mut self) -> Result<Program, ParseError> {
        let mut imports = Vec::new();
        let mut databases = Vec::new();
        let mut tables = Vec::new();
        let mut types = Vec::new();
        let mut records = Vec::new();
        let mut enums = Vec::new();
        let mut views = Vec::new();
        let mut components = Vec::new();
        let mut pages = Vec::new();
        let mut tableviews = Vec::new();
        let mut forms = Vec::new();
        let mut cruds = Vec::new();
        let mut auth = Vec::new();
        let mut apis = Vec::new();
        let mut commands = Vec::new();
        let mut schedules = Vec::new();
        let mut functions = Vec::new();
        self.skip_newlines();
        let mut declarations_started = false;
        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Import) {
                if declarations_started {
                    return self.error("module imports must appear before declarations");
                }
                imports.push(self.import_definition()?);
            } else if self.at(&TokenKind::Pub) {
                declarations_started = true;
                self.advance();
                if self.at(&TokenKind::Fn) {
                    functions.push(self.function(true)?);
                } else if self.at(&TokenKind::Type) {
                    types.push(self.type_definition(true)?);
                } else if self.at(&TokenKind::Struct) || self.at_word("class") {
                    records.push(self.record_definition(true)?);
                } else if self.at(&TokenKind::Enum) {
                    enums.push(self.enum_definition(true)?);
                } else if self.at(&TokenKind::View) {
                    views.push(self.view_definition(true)?);
                } else if self.at(&TokenKind::Component) {
                    components.push(self.component_definition(true)?);
                } else {
                    return self
                        .error("`pub` currently applies only to functions, types, enums, records, classes, views, and components");
                }
            } else if self.at(&TokenKind::Database) {
                declarations_started = true;
                databases.push(self.database_definition()?);
            } else if self.at(&TokenKind::Table) {
                declarations_started = true;
                tables.push(self.table_definition()?);
            } else if self.at(&TokenKind::Type) {
                declarations_started = true;
                types.push(self.type_definition(false)?);
            } else if self.at(&TokenKind::Struct) || self.at_word("class") {
                declarations_started = true;
                records.push(self.record_definition(false)?);
            } else if self.at(&TokenKind::Enum) {
                declarations_started = true;
                enums.push(self.enum_definition(false)?);
            } else if self.at(&TokenKind::View) {
                declarations_started = true;
                views.push(self.view_definition(false)?);
            } else if self.at(&TokenKind::Component) {
                declarations_started = true;
                components.push(self.component_definition(false)?);
            } else if self.at(&TokenKind::Page) {
                declarations_started = true;
                pages.push(self.page_definition()?);
            } else if self.at(&TokenKind::TableView) {
                declarations_started = true;
                tableviews.push(self.tableview_definition()?);
            } else if self.at(&TokenKind::Form) {
                declarations_started = true;
                forms.push(self.form_definition()?);
            } else if self.at(&TokenKind::Crud) {
                declarations_started = true;
                cruds.push(self.crud_definition()?);
            } else if self.at(&TokenKind::Auth) {
                declarations_started = true;
                auth.push(self.auth_definition()?);
            } else if self.at(&TokenKind::Api) {
                declarations_started = true;
                apis.push(self.api_definition()?);
            } else if self.at_word("command") {
                declarations_started = true;
                commands.push(self.command_definition()?);
            } else if self.at_word("schedule") {
                declarations_started = true;
                schedules.push(self.schedule_definition()?);
            } else {
                declarations_started = true;
                functions.push(self.function(false)?);
            }
            self.skip_newlines();
        }
        Ok(Program {
            imports,
            databases,
            tables,
            types,
            records,
            enums,
            views,
            components,
            pages,
            tableviews,
            forms,
            cruds,
            auth,
            apis,
            commands,
            schedules,
            functions,
        })
    }

    fn command_definition(&mut self) -> Result<zelyra_ast::CommandDef, ParseError> {
        let start = self.advance().span;
        let (name, _) = self.ident("command name")?;
        self.expect(TokenKind::LBrace, "`{` after command name")?;
        let mut handler = None;
        let mut description = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (field, _) = if self.at(&TokenKind::Handler) {
                ("handler".to_owned(), self.advance().span)
            } else {
                self.ident("command property (`handler` or `description`)")?
            };
            self.expect(TokenKind::Colon, "colon after command property")?;
            match field.as_str() {
                "handler" if handler.is_none() => {
                    handler = Some(self.ident("command handler function")?.0);
                }
                "description" if description.is_none() => {
                    description = Some(self.string_value("command description")?);
                }
                "handler" | "description" => {
                    return self.error(format!(
                        "command property `{field}` is declared more than once"
                    ));
                }
                _ => return self.error(format!("unknown command property `{field}`")),
            }
            if self.at(&TokenKind::Semicolon) {
                self.advance();
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after command declaration")?;
        let Some(handler) = handler else {
            return self.error("command requires `handler`");
        };
        Ok(zelyra_ast::CommandDef {
            name,
            handler,
            description,
            span: start.join(end),
        })
    }

    fn schedule_definition(&mut self) -> Result<zelyra_ast::ScheduleDef, ParseError> {
        let start = self.advance().span;
        let (name, _) = self.ident("schedule name")?;
        self.expect(TokenKind::LBrace, "`{` after schedule name")?;
        let mut command = None;
        let mut cron = None;
        let mut timezone = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (field, _) = self.ident("schedule property (`command`, `cron`, or `timezone`)")?;
            self.expect(TokenKind::Colon, "colon after schedule property")?;
            match field.as_str() {
                "command" if command.is_none() => {
                    command = Some(self.ident("declared command name")?.0);
                }
                "cron" if cron.is_none() => {
                    cron = Some(self.string_value("five-field cron expression")?);
                }
                "timezone" if timezone.is_none() => {
                    timezone = Some(self.string_value("timezone name")?);
                }
                "command" | "cron" | "timezone" => {
                    return self.error(format!(
                        "schedule property `{field}` is declared more than once"
                    ));
                }
                _ => return self.error(format!("unknown schedule property `{field}`")),
            }
            if self.at(&TokenKind::Semicolon) {
                self.advance();
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after schedule declaration")?;
        let Some(command) = command else {
            return self.error("schedule requires `command`");
        };
        let Some(cron) = cron else {
            return self.error("schedule requires `cron`");
        };
        Ok(zelyra_ast::ScheduleDef {
            name,
            command,
            cron,
            timezone,
            span: start.join(end),
        })
    }

    fn import_definition(&mut self) -> Result<ImportDef, ParseError> {
        let start = self.expect(TokenKind::Import, "`import`")?;
        let path = self.string_value("project-relative module path")?;
        let (as_keyword, _) = self.ident("`as` after module path")?;
        if as_keyword != "as" {
            return self.error("expected `as` after module path");
        }
        let (alias, alias_span) = self.ident("module alias")?;
        Ok(ImportDef {
            path,
            alias,
            span: start.join(alias_span),
        })
    }

    fn api_definition(&mut self) -> Result<ApiDef, ParseError> {
        let start = self.expect(TokenKind::Api, "`api`")?;
        let method = self
            .ident("HTTP method after `api`")?
            .0
            .to_ascii_uppercase();
        if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
            return self.error(format!("unsupported HTTP method `{method}`"));
        }
        let path = self.string_value("API path")?;
        self.expect(TokenKind::LBrace, "`{` after API path")?;
        let mut handler = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        let mut input = Vec::new();
        let mut validates_table = None;
        let mut output = None;
        let mut errors = Vec::new();
        let mut version = None;
        let mut deprecated = false;
        let mut saw_deprecated = false;
        let mut rate_limit = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Handler) {
                self.advance();
                handler = Some(self.ident("API handler function name")?.0);
            } else if self.at(&TokenKind::Requires) {
                self.advance();
                self.expect(TokenKind::Auth, "auth after requires")?;
                requires_auth = true;
            } else if self.at(&TokenKind::Permits) {
                self.advance();
                permissions.push(self.string_value("API permission")?);
            } else if self.at_word("validates") {
                let (_, validates_span) = self.ident("`validates`")?;
                if validates_table.is_some() {
                    return Err(ParseError {
                        message: "API validation table may only be declared once".into(),
                        span: validates_span,
                    });
                }
                validates_table = Some(self.ident("table name after `validates`")?.0);
            } else if self.at(&TokenKind::Version) {
                self.advance();
                if version.is_some() {
                    return self.error("API version may only be declared once");
                }
                let value = self.string_value("API version")?;
                if value.is_empty()
                    || value.len() > 64
                    || !value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
                {
                    return self.error(
                        "API version must contain 1 to 64 ASCII letters, digits, dots, underscores, or hyphens",
                    );
                }
                version = Some(value);
            } else if self.at(&TokenKind::Deprecated) {
                self.advance();
                if saw_deprecated {
                    return self.error("API deprecation may only be declared once");
                }
                saw_deprecated = true;
                deprecated = true;
            } else if self.at(&TokenKind::RateLimit) {
                self.advance();
                if rate_limit.is_some() {
                    return self.error("API rate limit may only be declared once");
                }
                let requests = match self.current().kind.clone() {
                    TokenKind::Int(value) if (1..=1_000_000).contains(&value) => {
                        self.advance();
                        value as u32
                    }
                    _ => {
                        return self.error("API rate limit requests must be between 1 and 1000000")
                    }
                };
                let (per, _) = self.ident("`per` in API rate limit")?;
                if per != "per" {
                    return self.error("expected `per` before API rate limit window");
                }
                let window_seconds = match self.current().kind.clone() {
                    TokenKind::Int(value) if (1..=86_400).contains(&value) => {
                        self.advance();
                        value as u32
                    }
                    _ => {
                        return self
                            .error("API rate limit window must be between 1 and 86400 seconds")
                    }
                };
                rate_limit = Some(zelyra_ast::ApiRateLimit {
                    requests,
                    window_seconds,
                });
            } else if self.at(&TokenKind::Input) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `input`")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let (name, span) = self.ident("API input field name")?;
                    self.expect(TokenKind::Colon, "`:` after API input field name")?;
                    let ty = self.type_name()?;
                    input.push(ApiField { name, ty, span });
                    self.skip_newlines();
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }
                self.expect(TokenKind::RBrace, "`}` after API input")?;
            } else if self.at(&TokenKind::Output) {
                self.advance();
                output = Some(self.type_name()?);
            } else if self.at(&TokenKind::Errors) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `errors`")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let status = match self.current().kind.clone() {
                        TokenKind::Int(value) if (100..=599).contains(&value) => {
                            self.advance();
                            value as u16
                        }
                        _ => return self.error("expected HTTP status code in API errors"),
                    };
                    let (name, span) = self.ident("API error name")?;
                    let payload = if self.at(&TokenKind::Colon) {
                        self.advance();
                        Some(self.type_name()?)
                    } else {
                        None
                    };
                    errors.push(ApiError {
                        status,
                        name,
                        payload,
                        span,
                    });
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace, "`}` after API errors")?;
            } else {
                return self
                    .error("expected `handler`, `requires auth`, `permits`, `validates`, `version`, `deprecated`, `rate_limit`, `input`, `output`, or `errors` in API definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after API definition")?;
        let Some(output) = output else {
            return self.error("API definition requires an `output` type");
        };
        Ok(ApiDef {
            method,
            path,
            handler,
            requires_auth,
            permissions,
            input,
            validates_table,
            output,
            errors,
            version,
            deprecated,
            rate_limit,
            span: start.join(end),
        })
    }

    fn auth_definition(&mut self) -> Result<AuthDef, ParseError> {
        let start = self.expect(TokenKind::Auth, "auth")?;
        let (name, _) = self.ident("authentication name")?;
        self.expect(TokenKind::LBrace, "opening brace after authentication name")?;
        self.skip_newlines();
        let mut table = None;
        let mut membership_table = None;
        let mut tenant_owner = None;
        let mut session_table = None;
        let mut permissions_table = None;
        let mut roles_table = None;
        let mut role_permissions_table = None;
        let mut audit_table = None;
        let mut audit_chain = false;
        let mut admin_path = None;
        let mut admin_permission = None;
        let mut admin_role = None;
        let mut login_rate_limit = None;
        let mut login_block_seconds = None;
        let mut reset_tokens_table = None;
        let mut reset_rate_limits_table = None;
        let mut reset_rate_limit = None;
        let mut reset_block_seconds = None;
        let mut mfa_table = None;
        let mut mfa_recovery_table = None;
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let field = match self.current().kind.clone() {
                TokenKind::Table => "table",
                TokenKind::Sessions => "sessions",
                TokenKind::Permissions => "permissions",
                TokenKind::Roles => "roles",
                TokenKind::RolePermissions => "role_permissions",
                TokenKind::Audit => "audit",
                TokenKind::AuditChain => "audit_chain",
                TokenKind::AdminPath => "admin_path",
                TokenKind::AdminPermission => "admin_permission",
                TokenKind::AdminRole => "admin_role",
                TokenKind::LoginRateLimit => "login_rate_limit",
                TokenKind::LoginBlockSeconds => "login_block_seconds",
                TokenKind::ResetTokens => "reset_tokens",
                TokenKind::Ident(name) if name == "reset_rate_limits" => "reset_rate_limits",
                TokenKind::Ident(name) if name == "memberships" => "memberships",
                TokenKind::Ident(name) if name == "tenant_owner" => "tenant_owner",
                TokenKind::ResetRateLimit => "reset_rate_limit",
                TokenKind::ResetBlockSeconds => "reset_block_seconds",
                TokenKind::Ident(name) if name == "mfa" => "mfa",
                TokenKind::Ident(name) if name == "mfa_recovery" => "mfa_recovery",
                _ => return self.error("expected authentication option"),
            };
            self.advance();
            self.expect(TokenKind::Colon, "colon after authentication option")?;
            if field == "tenant_owner" {
                if tenant_owner.is_some() {
                    return self.error("tenant_owner may only be declared once");
                }
                let (owner_table, _) = self.ident("tenant owner table")?;
                self.expect(TokenKind::Dot, "`.` before tenant owner column")?;
                let (owner_column, _) = self.ident("tenant owner column")?;
                tenant_owner = Some((owner_table, owner_column));
                self.skip_newlines();
                continue;
            }
            if field == "audit_chain" {
                audit_chain = match self.current().kind.clone() {
                    TokenKind::True => {
                        self.advance();
                        true
                    }
                    TokenKind::False => {
                        self.advance();
                        false
                    }
                    _ => return self.error("expected `true` or `false` for audit_chain"),
                };
                self.skip_newlines();
                continue;
            }
            if matches!(field, "login_rate_limit" | "reset_rate_limit") {
                let rate_limit = if field == "login_rate_limit" {
                    &mut login_rate_limit
                } else {
                    &mut reset_rate_limit
                };
                if rate_limit.is_some() {
                    return self.error(format!("{field} may only be declared once"));
                }
                let requests = match self.current().kind.clone() {
                    TokenKind::Int(value) if (1..=1_000).contains(&value) => {
                        self.advance();
                        value as u32
                    }
                    _ => return self.error(format!("{field} attempts must be between 1 and 1000")),
                };
                let (per, _) = self.ident(&format!("`per` in {field}"))?;
                if per != "per" {
                    return self.error(format!("expected `per` before {field} window"));
                }
                let window_seconds = match self.current().kind.clone() {
                    TokenKind::Int(value) if (1..=86_400).contains(&value) => {
                        self.advance();
                        value as u32
                    }
                    _ => {
                        return self.error(format!(
                            "{field} window must be between 1 and 86400 seconds"
                        ))
                    }
                };
                *rate_limit = Some(zelyra_ast::ApiRateLimit {
                    requests,
                    window_seconds,
                });
                self.skip_newlines();
                continue;
            }
            if matches!(field, "login_block_seconds" | "reset_block_seconds") {
                let block_seconds = if field == "login_block_seconds" {
                    &mut login_block_seconds
                } else {
                    &mut reset_block_seconds
                };
                if block_seconds.is_some() {
                    return self.error(format!("{field} may only be declared once"));
                }
                *block_seconds = match self.current().kind.clone() {
                    TokenKind::Int(value) if (1..=86_400).contains(&value) => {
                        self.advance();
                        Some(value as u32)
                    }
                    _ => return self.error(format!("{field} must be between 1 and 86400")),
                };
                self.skip_newlines();
                continue;
            }
            let value = if matches!(field, "admin_path" | "admin_permission") {
                self.string_value("authentication option value")?
            } else {
                self.ident("authentication option value")?.0
            };
            match field {
                "table" => table = Some(value),
                "memberships" => membership_table = Some(value),
                "sessions" => session_table = Some(value),
                "permissions" => permissions_table = Some(value),
                "roles" => roles_table = Some(value),
                "role_permissions" => role_permissions_table = Some(value),
                "audit" => audit_table = Some(value),
                "admin_path" => admin_path = Some(value),
                "admin_permission" => admin_permission = Some(value),
                "admin_role" => admin_role = Some(value),
                "reset_tokens" => reset_tokens_table = Some(value),
                "reset_rate_limits" => {
                    if reset_rate_limits_table.is_some() {
                        return self.error("reset_rate_limits may only be declared once");
                    }
                    reset_rate_limits_table = Some(value);
                }
                "mfa" => mfa_table = Some(value),
                "mfa_recovery" => mfa_recovery_table = Some(value),
                _ => return self.error("unknown authentication option"),
            }
            self.skip_newlines();
        }
        let end = self.expect(
            TokenKind::RBrace,
            "closing brace after authentication definition",
        )?;
        let Some(table) = table else {
            return self.error("authentication definition requires a table option");
        };
        Ok(AuthDef {
            name,
            table,
            membership_table,
            tenant_owner_table: tenant_owner.as_ref().map(|(table, _)| table.clone()),
            tenant_owner_column: tenant_owner.map(|(_, column)| column),
            session_table,
            permissions_table,
            roles_table,
            role_permissions_table,
            audit_table,
            audit_chain,
            admin_path,
            admin_permission,
            admin_role,
            login_rate_limit,
            login_block_seconds,
            reset_tokens_table,
            reset_rate_limits_table,
            reset_rate_limit,
            reset_block_seconds,
            mfa_table,
            mfa_recovery_table,
            span: start.join(end),
        })
    }

    fn crud_definition(&mut self) -> Result<CrudDef, ParseError> {
        let start = self.expect(TokenKind::Crud, "`crud`")?;
        let (name, _) = self.ident("CRUD resource name")?;
        self.expect(TokenKind::Arrow, "`->` after CRUD resource name")?;
        let (table, table_span) = self.ident("table name after `->`")?;
        let mut tenant_column = None;
        let mut recipient_column = None;
        let mut unread_column = None;
        let mut expires_column = None;
        let mut title = None;
        let mut layout = None;
        let mut layout_slots = Vec::new();
        let mut list = Vec::new();
        let mut search = Vec::new();
        let mut filters = Vec::new();
        let mut view = CrudViewDef::default();
        let mut soft_delete = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        let mut create_permissions = Vec::new();
        let mut edit_permissions = Vec::new();
        let mut delete_permissions = Vec::new();
        let mut conditional_permissions = Vec::new();
        let mut guards = Vec::new();
        let mut children = Vec::new();
        let mut actions = Vec::new();

        if self.at(&TokenKind::LBrace) {
            self.advance();
            self.skip_newlines();
            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                match self.current().kind.clone() {
                    TokenKind::Ident(name) if name == "tenant" => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD tenant")?;
                        tenant_column = Some(self.ident("tenant key column")?.0);
                    }
                    TokenKind::Ident(name) if name == "recipient" => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD recipient")?;
                        recipient_column = Some(self.ident("recipient user ID column")?.0);
                    }
                    TokenKind::Ident(name) if name == "unread" => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD unread marker")?;
                        unread_column = Some(self.ident("nullable read timestamp column")?.0);
                    }
                    TokenKind::Ident(name) if name == "expires" => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD expiry")?;
                        expires_column = Some(self.ident("expiry timestamp column")?.0);
                    }
                    TokenKind::Title => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD title")?;
                        title = Some(self.string_value("CRUD title")?);
                    }
                    TokenKind::Layout => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD layout")?;
                        layout = Some(self.ident("CRUD layout view name")?.0);
                    }
                    TokenKind::Ident(name) if name == "slots" => {
                        self.advance();
                        layout_slots.extend(self.crud_layout_slots_block()?);
                    }
                    TokenKind::List => {
                        self.advance();
                        list = self.crud_column_block("list")?;
                    }
                    TokenKind::Search => {
                        self.advance();
                        search = self.crud_column_block("search")?;
                    }
                    TokenKind::Filter => {
                        self.advance();
                        filters = self.crud_column_block("filter")?;
                    }
                    TokenKind::View => {
                        self.advance();
                        view = self.crud_view_block()?;
                    }
                    TokenKind::SoftDelete => {
                        self.advance();
                        soft_delete = Some(self.crud_soft_delete_block()?);
                    }
                    TokenKind::Requires => {
                        self.advance();
                        self.expect(TokenKind::Auth, "auth after requires")?;
                        requires_auth = true;
                    }
                    TokenKind::Permits => {
                        self.advance();
                        let scope = match &self.current().kind {
                            TokenKind::Ident(scope)
                                if matches!(scope.as_str(), "view" | "create" | "edit" | "delete") =>
                            {
                                let scope = scope.clone();
                                self.advance();
                                Some(scope)
                            }
                            TokenKind::View => {
                                self.advance();
                                Some("view".into())
                            }
                            _ => None,
                        };
                        let permission = self.string_value("permission")?;
                        if self.at_word("when") {
                            let (_, when_span) = self.ident("`when`")?;
                            let Some(action) = crud_record_action(scope.as_deref()) else {
                                return Err(ParseError {
                                    message: "conditional CRUD permissions require `permits edit` or `permits delete`".into(),
                                    span: when_span,
                                });
                            };
                            let condition = self.expression()?;
                            conditional_permissions.push(zelyra_ast::CrudConditionalPermissionDef {
                                action,
                                permission,
                                span: when_span.join(condition.span),
                                condition,
                            });
                        } else {
                            match scope.as_deref() {
                                Some("create") => create_permissions.push(permission),
                                Some("edit") => edit_permissions.push(permission),
                                Some("delete") => delete_permissions.push(permission),
                                Some("view") | None => permissions.push(permission),
                                _ => unreachable!("CRUD permission scope was validated above"),
                            }
                        }
                    }
                    TokenKind::Ident(name) if name == "guard" => {
                        guards.push(self.crud_guard_definition()?);
                    }
                    TokenKind::Ident(name) if name == "has_many" => {
                        let child = self.crud_child_relation_definition()?;
                        if children.iter().any(|existing: &zelyra_ast::CrudChildRelationDef| existing.name == child.name) {
                            return Err(ParseError {
                                message: format!("duplicate CRUD child relation `{}`", child.name),
                                span: child.span,
                            });
                        }
                        children.push(child);
                    }
                    TokenKind::Action => {
                        actions.push(self.form_action()?);
                    }
                    _ => {
                        return self.error(
                            "expected title, layout, slots, tenant, recipient, unread, expires, list, search, filter, view, requires auth, permits, guard, has_many, or action in CRUD definition",
                        )
                    }
                }
                self.skip_newlines();
            }
            let end = self.expect(TokenKind::RBrace, "`}` after CRUD definition")?;
            return Ok(CrudDef {
                name,
                table,
                tenant_column,
                recipient_column,
                unread_column,
                expires_column,
                title,
                layout,
                layout_slots,
                list,
                search,
                filters,
                view,
                soft_delete,
                requires_auth,
                permissions,
                create_permissions,
                edit_permissions,
                delete_permissions,
                conditional_permissions,
                guards,
                children,
                actions,
                span: start.join(end),
            });
        }

        Ok(CrudDef {
            name,
            table,
            tenant_column,
            recipient_column,
            unread_column,
            expires_column,
            title,
            layout,
            layout_slots,
            list,
            search,
            filters,
            view,
            soft_delete,
            requires_auth,
            permissions,
            create_permissions,
            edit_permissions,
            delete_permissions,
            conditional_permissions,
            guards,
            children,
            actions,
            span: start.join(table_span),
        })
    }

    fn crud_child_relation_definition(
        &mut self,
    ) -> Result<zelyra_ast::CrudChildRelationDef, ParseError> {
        let (_, start) = self.ident("`has_many`")?;
        let (name, _) = self.ident("child collection name")?;
        self.expect(TokenKind::Colon, "`:` after child collection name")?;
        let (model, _) = self.ident("child model name")?;
        self.expect(TokenKind::LBrace, "`{` after child model")?;
        let mut foreign_key = None;
        let mut order_by = None;
        let mut descending = false;
        let mut on_delete = None;
        let mut aggregates = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (property, property_span) = self.ident("child relation property")?;
            match property.as_str() {
                "foreign_key" => {
                    self.expect(TokenKind::Colon, "`:` after `foreign_key`")?;
                    if foreign_key.replace(self.ident("child foreign-key field")?.0).is_some() {
                        return Err(ParseError { message: "duplicate `foreign_key` in CRUD child relation".into(), span: property_span });
                    }
                }
                "order_by" => {
                    self.expect(TokenKind::Colon, "`:` after `order_by`")?;
                    let (column, _) = self.ident("child ordering field")?;
                    if order_by.replace(column).is_some() {
                        return Err(ParseError { message: "duplicate `order_by` in CRUD child relation".into(), span: property_span });
                    }
                    let direction = self.ident("`asc` or `desc` after child ordering field")?.0;
                    descending = match direction.as_str() {
                        "asc" => false,
                        "desc" => true,
                        _ => return self.error("child ordering direction must be `asc` or `desc`"),
                    };
                }
                "cascade" => {
                    self.expect(TokenKind::Colon, "`:` after `cascade`")?;
                    let value = self.ident("`delete` or `restrict` after `cascade`")?.0;
                    let action = match value.as_str() {
                        "delete" => zelyra_ast::ReferentialAction::Cascade,
                        "restrict" => zelyra_ast::ReferentialAction::Restrict,
                        _ => return self.error("CRUD child cascade must be `delete` or `restrict`"),
                    };
                    if on_delete.replace(action).is_some() {
                        return Err(ParseError { message: "duplicate `cascade` in CRUD child relation".into(), span: property_span });
                    }
                }
                "aggregates" => {
                    self.expect(TokenKind::LBrace, "`{` after `aggregates`")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (target, target_span) = self.ident("aggregate target column")?;
                        self.expect(TokenKind::Colon, "`:` after aggregate target")?;
                        let (function, function_span) = self.ident("aggregate function `sum`")?;
                        if function != "sum" {
                            return Err(ParseError { message: "supported CRUD aggregate function is `sum`".into(), span: function_span });
                        }
                        self.expect(TokenKind::LParen, "`(` after aggregate function")?;
                        let expression = self.expression()?;
                        let end = self.expect(TokenKind::RParen, "`)` after aggregate expression")?;
                        if aggregates.iter().any(|entry: &zelyra_ast::CrudAggregateDef| entry.target == target) {
                            return Err(ParseError { message: format!("duplicate aggregate target `{target}`"), span: target_span });
                        }
                        aggregates.push(zelyra_ast::CrudAggregateDef {
                            target,
                            expression,
                            span: target_span.join(end),
                        });
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after child aggregates")?;
                }
                _ => return self.error("expected `foreign_key`, `order_by`, `cascade`, or `aggregates` in CRUD child relation"),
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after CRUD child relation")?;
        let Some(foreign_key) = foreign_key else {
            return Err(ParseError {
                message: "CRUD child relation requires `foreign_key`".into(),
                span: start.join(end),
            });
        };
        Ok(zelyra_ast::CrudChildRelationDef {
            name,
            model,
            foreign_key,
            order_by,
            descending,
            on_delete,
            aggregates,
            span: start.join(end),
        })
    }

    fn crud_guard_definition(&mut self) -> Result<zelyra_ast::CrudGuardDef, ParseError> {
        let (_, start) = self.ident("`guard`")?;
        let (action_name, action_span) = self.ident("`edit` or `delete` after `guard`")?;
        let action = match action_name.as_str() {
            "edit" => zelyra_ast::CrudRecordAction::Edit,
            "delete" => zelyra_ast::CrudRecordAction::Delete,
            _ => {
                return Err(ParseError {
                    message: "CRUD guards support only `edit` and `delete`".into(),
                    span: action_span,
                })
            }
        };
        let (when_name, when_span) = self.ident("`when` after CRUD guard action")?;
        if when_name != "when" {
            return Err(ParseError {
                message: "expected `when` after CRUD guard action".into(),
                span: when_span,
            });
        }
        let condition = self.expression()?;
        self.expect(TokenKind::LBrace, "`{` before CRUD guard error")?;
        self.skip_newlines();
        let (error_name, error_span) = self.ident("`error` in CRUD guard")?;
        if error_name != "error" {
            return Err(ParseError {
                message: "CRUD guard block requires `error: \"message\"`".into(),
                span: error_span,
            });
        }
        self.expect(TokenKind::Colon, "`:` after CRUD guard error")?;
        let message = self.string_value("CRUD guard error message")?;
        self.skip_newlines();
        let end = self.expect(TokenKind::RBrace, "`}` after CRUD guard error")?;
        Ok(zelyra_ast::CrudGuardDef {
            action,
            condition,
            message,
            span: start.join(end),
        })
    }

    fn crud_layout_slots_block(
        &mut self,
    ) -> Result<Vec<zelyra_ast::CrudLayoutSlotDef>, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after CRUD layout slots")?;
        let mut slots = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (name, start) = self.ident("CRUD layout slot name")?;
            self.expect(TokenKind::LBrace, "`{` after CRUD layout slot name")?;
            self.skip_newlines();
            self.expect(TokenKind::Html, "`html` in CRUD layout slot")?;
            self.skip_newlines();
            self.expect(TokenKind::LBrace, "`{` after CRUD layout slot `html`")?;
            let html = match self.current().kind.clone() {
                TokenKind::HtmlBody(body) => {
                    self.advance();
                    body
                }
                _ => return self.error("expected HTML body in CRUD layout slot"),
            };
            self.expect(TokenKind::RBrace, "`}` after CRUD layout slot HTML")?;
            self.skip_newlines();
            let end = self.expect(TokenKind::RBrace, "`}` after CRUD layout slot")?;
            slots.push(zelyra_ast::CrudLayoutSlotDef {
                name,
                html,
                span: start.join(end),
            });
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after CRUD layout slots")?;
        Ok(slots)
    }

    fn crud_column_block(&mut self, label: &str) -> Result<Vec<String>, ParseError> {
        let (open_label, close_label) = match label {
            "list" => ("`{` after CRUD list", "`}` after CRUD list"),
            "search" => ("`{` after CRUD search", "`}` after CRUD search"),
            _ => ("`{` after CRUD filter", "`}` after CRUD filter"),
        };
        self.expect(TokenKind::LBrace, open_label)?;
        let mut columns = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            columns.push(self.ident("CRUD column name")?.0);
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        self.expect(TokenKind::RBrace, close_label)?;
        Ok(columns)
    }

    fn crud_soft_delete_block(&mut self) -> Result<CrudSoftDeleteDef, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after soft_delete")?;
        let mut column = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (property, _) = self.ident("soft_delete property")?;
            self.expect(TokenKind::Colon, "colon after soft_delete property")?;
            match property.as_str() {
                "column" => column = Some(self.ident("soft_delete column")?.0),
                _ => return self.error("expected `column` in soft_delete definition"),
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after soft_delete")?;
        let Some(column) = column else {
            return self.error("soft_delete definition requires a column");
        };
        Ok(CrudSoftDeleteDef { column })
    }

    fn crud_view_block(&mut self) -> Result<CrudViewDef, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after CRUD view")?;
        let mut view = CrudViewDef::default();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            match self.current().kind.clone() {
                TokenKind::Fields => {
                    self.advance();
                    view.fields = self.crud_view_field_block()?;
                }
                TokenKind::List => {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after CRUD view list")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (property, _) = self.ident("CRUD list view property")?;
                        self.expect(TokenKind::Colon, "colon after CRUD list view property")?;
                        match property.as_str() {
                            "mode" => {
                                let (mode, _) = self.ident("CRUD list view mode")?;
                                view.list.mode = match mode.as_str() {
                                    "table" => CrudListViewMode::Table,
                                    "cards" => CrudListViewMode::Cards,
                                    _ => {
                                        return self.error(
                                            "CRUD list view mode must be `table` or `cards`",
                                        )
                                    }
                                };
                            }
                            "empty" => {
                                view.list.empty =
                                    Some(self.string_value("CRUD list empty message")?);
                            }
                            _ => {
                                return self.error(
                                    "expected `mode` or `empty` in CRUD list view definition",
                                )
                            }
                        }
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD view list")?;
                }
                TokenKind::Detail => {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after CRUD view detail")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (property, _) = self.ident("CRUD detail view property")?;
                        self.expect(TokenKind::Colon, "colon after CRUD detail view property")?;
                        match property.as_str() {
                            "mode" => {
                                let (mode, _) = self.ident("CRUD detail view mode")?;
                                view.detail.mode =
                                    match mode.as_str() {
                                        "standard" => CrudDetailViewMode::Standard,
                                        "cards" => CrudDetailViewMode::Cards,
                                        _ => return self.error(
                                            "CRUD detail view mode must be `standard` or `cards`",
                                        ),
                                    };
                            }
                            "title" => {
                                view.detail.title =
                                    Some(self.string_value("CRUD detail view title")?);
                            }
                            _ => {
                                return self.error(
                                    "expected `mode` or `title` in CRUD detail view definition",
                                )
                            }
                        }
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD view detail")?;
                }
                TokenKind::Form => {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after CRUD view form")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (property, _) = self.ident("CRUD form view property")?;
                        self.expect(TokenKind::Colon, "colon after CRUD form view property")?;
                        match property.as_str() {
                            "mode" => {
                                let (mode, _) = self.ident("CRUD form view mode")?;
                                view.form.mode = match mode.as_str() {
                                    "standard" => CrudFormViewMode::Standard,
                                    "cards" => CrudFormViewMode::Cards,
                                    _ => {
                                        return self.error(
                                            "CRUD form view mode must be `standard` or `cards`",
                                        )
                                    }
                                };
                            }
                            "title" => {
                                view.form.title = Some(self.string_value("CRUD form view title")?);
                            }
                            "submit" => {
                                view.form.submit =
                                    Some(self.string_value("CRUD form submit label")?);
                            }
                            _ => {
                                return self.error(
                                    "expected `mode`, `title`, or `submit` in CRUD form view definition",
                                )
                            }
                        }
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD view form")?;
                }
                TokenKind::Ident(name) if name == "create" || name == "edit" => {
                    let is_create = name == "create";
                    self.advance();
                    let page = self.crud_form_page_block(if is_create { "create" } else { "edit" })?;
                    if is_create {
                        if view.create.span != Span::default() {
                            return self.error("duplicate CRUD `create` view block");
                        }
                        view.create = page;
                    } else {
                        if view.edit.span != Span::default() {
                            return self.error("duplicate CRUD `edit` view block");
                        }
                        view.edit = page;
                    }
                }
                TokenKind::Ident(name) if name == "delete" => {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after CRUD view delete")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (property, _) = self.ident("CRUD delete view property")?;
                        self.expect(TokenKind::Colon, "colon after CRUD delete view property")?;
                        match property.as_str() {
                            "title" => {
                                view.delete.title =
                                    Some(self.string_value("CRUD delete view title")?);
                            }
                            "message" => {
                                view.delete.message =
                                    Some(self.string_value("CRUD delete view message")?);
                            }
                            "submit" => {
                                view.delete.submit =
                                    Some(self.string_value("CRUD delete submit label")?);
                            }
                            _ => {
                                return self.error(
                                    "expected `title`, `message`, or `submit` in CRUD delete view definition",
                                )
                            }
                        }
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD view delete")?;
                }
                TokenKind::Ident(name) if name == "loading" => {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after CRUD view loading")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (property, _) = self.ident("CRUD loading view property")?;
                        self.expect(TokenKind::Colon, "colon after CRUD loading view property")?;
                        if property != "message" {
                            return self.error(
                                "expected `message` in CRUD loading view definition",
                            );
                        }
                        view.loading.message =
                            Some(self.string_value("CRUD loading message")?);
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD view loading")?;
                }
                TokenKind::Ident(name) if name == "error" => {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after CRUD view error")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (property, _) = self.ident("CRUD error view property")?;
                        self.expect(TokenKind::Colon, "colon after CRUD error view property")?;
                        match property.as_str() {
                            "title" => {
                                view.error.title =
                                    Some(self.string_value("CRUD error view title")?);
                            }
                            "message" => {
                                view.error.message =
                                    Some(self.string_value("CRUD error view message")?);
                            }
                            _ => {
                                return self.error(
                                    "expected `title` or `message` in CRUD error view definition",
                                )
                            }
                        }
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD view error")?;
                }
                _ => {
                    return self.error(
                        "expected `fields`, `list`, `detail`, `form`, `create`, `edit`, `delete`, `loading`, or `error` in CRUD view definition",
                    )
                }
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after CRUD view definition")?;
        Ok(view)
    }

    fn crud_form_page_block(&mut self, page_name: &str) -> Result<CrudFormPageDef, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after CRUD page block")?;
        let start = self.current().span;
        let mut page = CrudFormPageDef::default();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (property, property_span) = self.ident("CRUD page property or content block")?;
            match property.as_str() {
                "title" | "submit" => {
                    self.expect(TokenKind::Colon, "`:` after CRUD page property")?;
                    let value = self.string_value("CRUD page text")?;
                    let target = if property == "title" { &mut page.title } else { &mut page.submit };
                    if target.replace(value).is_some() {
                        return self.error(format!("duplicate `{property}` in CRUD {page_name} view"));
                    }
                }
                "before_form" | "after_form" => {
                    let body = self.crud_page_html_block(&property)?;
                    let target = if property == "before_form" { &mut page.before_form } else { &mut page.after_form };
                    if target.replace(body).is_some() {
                        return self.error(format!("duplicate `{property}` in CRUD {page_name} view"));
                    }
                }
                "field_content" => {
                    self.expect(TokenKind::LBrace, "`{` after CRUD field_content")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (field, span) = self.ident("CRUD field name")?;
                        self.expect(TokenKind::LBrace, "`{` after field content name")?;
                        self.skip_newlines();
                        self.expect(TokenKind::Html, "`html` in CRUD field content")?;
                        self.expect(TokenKind::LBrace, "`{` after `html`")?;
                        let html = match self.current().kind.clone() {
                            TokenKind::HtmlBody(body) => { self.advance(); body }
                            _ => return self.error("expected HTML body in CRUD field content"),
                        };
                        self.expect(TokenKind::RBrace, "`}` after CRUD field HTML")?;
                        self.skip_newlines();
                        let end = self.expect(TokenKind::RBrace, "`}` after CRUD field content")?;
                        if page.field_content.iter().any(|entry| entry.field == field) {
                            return self.error(format!("duplicate field content for `{field}`"));
                        }
                        page.field_content.push(CrudFieldContentDef { field, html, span: span.join(end) });
                        self.skip_newlines();
                    }
                    self.expect(TokenKind::RBrace, "`}` after CRUD field_content")?;
                }
                _ => return self.error(format!("expected `title`, `submit`, `before_form`, `after_form`, or `field_content` in CRUD {page_name} view")),
            }
            if page.span == Span::default() {
                page.span = property_span;
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after CRUD page block")?;
        page.span = if start == Span::default() {
            end
        } else {
            start.join(end)
        };
        Ok(page)
    }

    fn crud_page_html_block(&mut self, _name: &str) -> Result<String, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after CRUD page content")?;
        self.skip_newlines();
        self.expect(TokenKind::Html, "`html` in CRUD page content")?;
        self.expect(TokenKind::LBrace, "`{` after `html`")?;
        let body = match self.current().kind.clone() {
            TokenKind::HtmlBody(body) => {
                self.advance();
                body
            }
            _ => return self.error("expected HTML body in CRUD page content"),
        };
        self.expect(TokenKind::RBrace, "`}` after HTML body")?;
        self.skip_newlines();
        self.expect(TokenKind::RBrace, "`}` after CRUD page content")?;
        Ok(body)
    }

    fn crud_view_field_block(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after CRUD view fields")?;
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            fields.push(self.ident("CRUD view field name")?.0);
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        self.expect(TokenKind::RBrace, "`}` after CRUD view fields")?;
        Ok(fields)
    }
    fn database_definition(&mut self) -> Result<DatabaseDef, ParseError> {
        let start = self.expect(TokenKind::Database, "`database`")?;
        let (name, _) = self.ident("database name")?;
        self.expect(TokenKind::LBrace, "`{` after database name")?;
        let mut engine = None;
        let mut database = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Engine) {
                self.advance();
                self.expect(TokenKind::Colon, "`:` after engine")?;
                engine = Some(self.database_value("database engine")?);
            } else if self.at(&TokenKind::Database) {
                self.advance();
                self.expect(TokenKind::Colon, "`:` after database")?;
                database = Some(self.string_value("database name")?);
            } else {
                return self.error("expected `engine` or `database` in database definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after database definition")?;
        Ok(DatabaseDef {
            name,
            engine: engine.unwrap_or_else(|| "mariadb".into()),
            database,
            span: start.join(end),
        })
    }
    fn database_value(&mut self, label: &str) -> Result<String, ParseError> {
        if self.at(&TokenKind::Postgres) {
            self.advance();
            Ok("postgres".into())
        } else {
            self.ident(label).map(|(name, _)| name)
        }
    }
    fn string_value(&mut self, label: &str) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.advance();
                Ok(value)
            }
            TokenKind::StringTemplate { value, .. } => {
                let value = value.clone();
                self.advance();
                Ok(value)
            }
            _ => self.error(format!("expected {label}")),
        }
    }
    fn page_definition(&mut self) -> Result<PageDef, ParseError> {
        let start = self.expect(TokenKind::Page, "`page`")?;
        let path = self.string_value("page path")?;
        self.expect(TokenKind::LBrace, "`{` after page path")?;
        let mut html = None;
        let mut view = None;
        let mut inputs = Vec::new();
        let mut page_size = None;
        let mut sort = Vec::new();
        let mut search = Vec::new();
        let mut filters = Vec::new();
        let mut data = Vec::new();
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Input) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after page input")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let (name, span) = self.page_input_name()?;
                    self.expect(TokenKind::Colon, "`:` after page input name")?;
                    let ty = self.type_name()?;
                    inputs.push(PageInputDef { name, ty, span });
                    self.skip_newlines();
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }
                self.expect(TokenKind::RBrace, "`}` after page input")?;
            } else if self.at(&TokenKind::View) {
                self.advance();
                self.expect(TokenKind::Colon, "`:` after `view`")?;
                view = Some(self.ident("view name")?.0);
            } else if self.at(&TokenKind::Paginated) {
                self.advance();
                let value = match self.current().kind.clone() {
                    TokenKind::Int(value) => value,
                    _ => return self.error("expected page size after `paginated`"),
                };
                self.advance();
                if !(1..=100).contains(&value) {
                    return self.error("page size must be between 1 and 100");
                }
                page_size = Some(value as u32);
            } else if self.at(&TokenKind::Sort) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after page sort")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    sort.push(self.ident("page sort field")?.0);
                    self.skip_newlines();
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }
                self.expect(TokenKind::RBrace, "`}` after page sort")?;
            } else if self.at(&TokenKind::Search) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after page search")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    search.push(self.ident("page search field")?.0);
                    self.skip_newlines();
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }
                self.expect(TokenKind::RBrace, "`}` after page search")?;
            } else if self.at(&TokenKind::Filter) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after page filter")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    filters.push(self.ident("page filter field")?.0);
                    self.skip_newlines();
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }
                self.expect(TokenKind::RBrace, "`}` after page filter")?;
            } else if matches!(&self.current().kind, TokenKind::Ident(name) if name == "load") {
                let load_start = self.advance().span;
                let (name, _) = self.ident("page data name")?;
                self.expect(TokenKind::Equal, "`=` after page data name")?;
                let sql_start = self.expect(TokenKind::Sql, "`sql` after page data name")?;
                let expression = self.sql_expression(sql_start)?;
                let ExprKind::Sql { result_type, query } = expression.kind else {
                    return self.error("page data must use a SQL query");
                };
                data.push(PageDataDef {
                    name,
                    result_type,
                    query,
                    span: load_start.join(expression.span),
                });
            } else if self.at(&TokenKind::Html) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `html`")?;
                let body = match self.current().kind.clone() {
                    TokenKind::HtmlBody(body) => {
                        self.advance();
                        body
                    }
                    _ => return self.error("expected HTML body"),
                };
                self.expect(TokenKind::RBrace, "`}` after HTML body")?;
                html = Some(body);
            } else if self.at(&TokenKind::Requires) {
                self.advance();
                self.expect(TokenKind::Auth, "auth after requires")?;
                requires_auth = true;
            } else if self.at(&TokenKind::Permits) {
                self.advance();
                permissions.push(self.string_value("permission")?);
            } else {
                return self.error("expected `load`, `html`, `view`, `paginated`, `sort`, `search`, `filter`, `requires auth`, or `permits` in page definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after page definition")?;
        let Some(html) = html else {
            return self.error("page definition requires an `html` block");
        };
        Ok(PageDef {
            path,
            html,
            view,
            inputs,
            page_size,
            sort,
            search,
            filters,
            data,
            requires_auth,
            permissions,
            span: start.join(end),
        })
    }

    fn page_input_name(&mut self) -> Result<(String, Span), ParseError> {
        match self.current().kind.clone() {
            TokenKind::Ident(name) => {
                let span = self.advance().span;
                Ok((name, span))
            }
            TokenKind::Search => {
                let span = self.advance().span;
                Ok(("search".into(), span))
            }
            TokenKind::Page => {
                let span = self.advance().span;
                Ok(("page".into(), span))
            }
            _ => self.error("expected page input name"),
        }
    }

    fn view_definition(&mut self, is_public: bool) -> Result<ViewDef, ParseError> {
        let start = self.expect(TokenKind::View, "`view`")?;
        let (name, _) = self.ident("view name")?;
        self.expect(TokenKind::LBrace, "`{` after view name")?;
        let mut html = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Html) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `html`")?;
                let body = match self.current().kind.clone() {
                    TokenKind::HtmlBody(body) => {
                        self.advance();
                        body
                    }
                    _ => return self.error("expected HTML body"),
                };
                self.expect(TokenKind::RBrace, "`}` after HTML body")?;
                html = Some(body);
            } else {
                return self.error("expected `html` in view definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after view definition")?;
        let Some(html) = html else {
            return self.error("view definition requires an `html` block");
        };
        Ok(ViewDef {
            name,
            is_public,
            html,
            span: start.join(end),
        })
    }

    fn tableview_definition(&mut self) -> Result<TableViewDef, ParseError> {
        let start = self.expect(TokenKind::TableView, "`tableview`")?;
        let (name, _) = self.ident("tableview name")?;
        self.expect(TokenKind::LBrace, "`{` after tableview name")?;
        let mut layout = None;
        let mut result_type = None;
        let mut source = None;
        let mut columns = Vec::new();
        let mut view = TableViewViewDef::default();
        let mut filters = Vec::new();
        let mut searchable = false;
        let mut sortable = false;
        let mut page_size = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Layout) {
                self.advance();
                self.expect(TokenKind::Colon, "colon after tableview layout")?;
                layout = Some(self.ident("tableview layout view name")?.0);
            } else if self.at(&TokenKind::Source) {
                self.advance();
                let sql_start = self.expect(TokenKind::Sql, "`sql` after `source`")?;
                let expression = self.sql_expression(sql_start)?;
                let ExprKind::Sql {
                    result_type: source_type,
                    query,
                } = expression.kind
                else {
                    return self.error("tableview source must be a SQL query");
                };
                result_type = Some(source_type);
                source = Some(query);
            } else if self.at(&TokenKind::View) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after tableview `view`")?;
                let mut row_fields_seen = false;
                let mut column_content_seen = false;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let (block_name, block_span) = self.ident("tableview view block")?;
                    match block_name.as_str() {
                        "before_content" | "after_content" => {
                            let body = self.crud_page_html_block(&block_name)?;
                            let target = if block_name == "before_content" { &mut view.before_content } else { &mut view.after_content };
                            if target.replace(body).is_some() {
                                return self.error(format!("duplicate `{block_name}` tableview block"));
                            }
                        }
                        "row_fields" => {
                            if row_fields_seen {
                                return self.error("duplicate tableview `row_fields` block");
                            }
                            row_fields_seen = true;
                            self.expect(TokenKind::LBrace, "`{` after tableview row_fields")?;
                            let mut fields = Vec::new();
                            self.skip_newlines();
                            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                                let (field, _) = self.ident("tableview row field")?;
                                if fields.contains(&field) {
                                    return self.error(format!("duplicate tableview row field `{field}`"));
                                }
                                fields.push(field);
                                self.skip_newlines();
                            }
                            self.expect(TokenKind::RBrace, "`}` after tableview row_fields")?;
                            view.row_fields = fields;
                        }
                        "column_content" => {
                            if column_content_seen {
                                return self.error("duplicate tableview `column_content` block");
                            }
                            column_content_seen = true;
                            self.expect(TokenKind::LBrace, "`{` after tableview column_content")?;
                            self.skip_newlines();
                            let (property, property_span) = self.ident("tableview column content binding property")?;
                            if property != "item" {
                                return self.error("expected `item: <binding>` in tableview column_content");
                            }
                            self.expect(TokenKind::Colon, "`:` after tableview column content row binding")?;
                            let item = self.ident("tableview column content row binding")?.0;
                            self.skip_newlines();
                            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                                let (column, column_span) = self.ident("tableview column name")?;
                                self.expect(TokenKind::LBrace, "`{` after tableview column name")?;
                                self.skip_newlines();
                                self.expect(TokenKind::Html, "`html` in tableview column content")?;
                                self.expect(TokenKind::LBrace, "`{` after `html`")?;
                                let html = match self.current().kind.clone() {
                                    TokenKind::HtmlBody(body) => { self.advance(); body }
                                    _ => return self.error("expected HTML body in tableview column content"),
                                };
                                self.expect(TokenKind::RBrace, "`}` after tableview column HTML")?;
                                self.skip_newlines();
                                let end = self.expect(TokenKind::RBrace, "`}` after tableview column content")?;
                                if view.column_content.iter().any(|entry| entry.column == column) {
                                    return self.error(format!("duplicate tableview column content for `{column}`"));
                                }
                                view.column_content.push(TableViewColumnContentDef {
                                    column,
                                    item: item.clone(),
                                    html,
                                    span: property_span.join(end).join(column_span),
                                });
                                self.skip_newlines();
                            }
                            self.expect(TokenKind::RBrace, "`}` after tableview column_content")?;
                        }
                        "row_actions" => {
                            self.expect(TokenKind::LBrace, "`{` after tableview row_actions")?;
                            let mut item = None;
                            let mut html = None;
                            let mut crud_model = None;
                            let mut edit_html = None;
                            let mut delete_html = None;
                            self.skip_newlines();
                            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                                let property = if self.at(&TokenKind::Html) {
                                    "html".to_owned()
                                } else if self.at(&TokenKind::Crud) {
                                    self.advance();
                                    "crud".to_owned()
                                } else {
                                    self.ident("row_actions property")?.0
                                };
                                match property.as_str() {
                                    "item" => {
                                        self.expect(TokenKind::Colon, "`:` after row_actions item")?;
                                        item = Some(self.ident("tableview row binding")?.0);
                                    }
                                    "crud" => {
                                        self.expect(TokenKind::Colon, "`:` after row_actions crud")?;
                                        crud_model = Some(self.ident("CRUD model name")?.0);
                                    }
                                    "edit" | "delete" => {
                                        let target = property.as_str();
                                        self.expect(TokenKind::LBrace, "`{` after row action operation")?;
                                        self.skip_newlines();
                                        self.expect(TokenKind::Html, "`html` in row action operation")?;
                                        self.expect(TokenKind::LBrace, "`{` after row action html")?;
                                        let body = match self.current().kind.clone() {
                                            TokenKind::HtmlBody(body) => { self.advance(); body }
                                            _ => return self.error("expected HTML body in row action operation"),
                                        };
                                        self.expect(TokenKind::RBrace, "`}` after row action HTML")?;
                                        self.skip_newlines();
                                        self.expect(TokenKind::RBrace, "`}` after row action operation")?;
                                        let slot = if target == "edit" { &mut edit_html } else { &mut delete_html };
                                        if slot.replace(body).is_some() {
                                            return self.error(format!("duplicate `{target}` row action"));
                                        }
                                    }
                                    "html" => {
                                        self.expect(TokenKind::Html, "`html` in tableview row_actions")?;
                                        self.expect(TokenKind::LBrace, "`{` after row_actions html")?;
                                        html = Some(match self.current().kind.clone() {
                                            TokenKind::HtmlBody(body) => { self.advance(); body }
                                            _ => return self.error("expected HTML body in tableview row_actions"),
                                        });
                                        self.expect(TokenKind::RBrace, "`}` after row_actions HTML")?;
                                    }
                                    _ => return self.error("expected `item`, `crud`, `html`, `edit`, or `delete` in tableview row_actions"),
                                }
                                self.skip_newlines();
                            }
                            let end = self.expect(TokenKind::RBrace, "`}` after tableview row_actions")?;
                            if view.row_actions.is_some() { return self.error("duplicate tableview `row_actions` block"); }
                            let Some(item) = item else { return self.error("row_actions requires `item`"); };
                            if html.is_none() && edit_html.is_none() && delete_html.is_none() {
                                return self.error("row_actions requires `html`, `edit`, or `delete` content");
                            }
                            if (edit_html.is_some() || delete_html.is_some()) && crud_model.is_none() {
                                return self.error("operation-specific row actions require `crud: Model`");
                            }
                            view.row_actions = Some(TableViewRowActionsDef {
                                item,
                                crud_model,
                                html: html.unwrap_or_default(),
                                edit_html,
                                delete_html,
                                span: block_span.join(end),
                            });
                        }
                        _ => return self.error("expected `before_content`, `after_content`, `row_fields`, `column_content`, or `row_actions` in tableview view"),
                    }
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace, "`}` after tableview view")?;
            } else if self.at(&TokenKind::Columns) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `columns`")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    columns.push(self.ident("tableview column name")?.0);
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace, "`}` after tableview columns")?;
            } else if self.at(&TokenKind::Filter) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after tableview filter")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    filters.push(self.ident("tableview filter column name")?.0);
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace, "`}` after tableview filter")?;
            } else if self.at(&TokenKind::Searchable) {
                self.advance();
                searchable = true;
            } else if self.at(&TokenKind::Sortable) {
                self.advance();
                sortable = true;
            } else if self.at(&TokenKind::Paginated) {
                self.advance();
                let value = match self.current().kind.clone() {
                    TokenKind::Int(value) => value,
                    _ => return self.error("expected page size after `paginated`"),
                };
                self.advance();
                if !(1..=100).contains(&value) {
                    return self.error("tableview page size must be between 1 and 100");
                }
                page_size = Some(value as u32);
            } else if self.at(&TokenKind::Requires) {
                self.advance();
                self.expect(TokenKind::Auth, "auth after requires")?;
                requires_auth = true;
            } else if self.at(&TokenKind::Permits) {
                self.advance();
                permissions.push(self.string_value("permission")?);
            } else {
                return self.error(
                    "expected layout, source, columns, view, filter, searchable, sortable, paginated, requires auth, or permits in tableview definition",
                );
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after tableview definition")?;
        let Some(result_type) = result_type else {
            return self.error("tableview requires a typed SQL source");
        };
        let Some(source) = source else {
            return self.error("tableview requires a SQL source");
        };
        if columns.is_empty() {
            return self.error("tableview requires at least one column");
        }
        Ok(TableViewDef {
            name,
            layout,
            result_type,
            source,
            columns,
            view,
            filters,
            searchable,
            sortable,
            page_size,
            requires_auth,
            permissions,
            span: start.join(end),
        })
    }

    fn component_definition(&mut self, is_public: bool) -> Result<ComponentDef, ParseError> {
        let start = self.expect(TokenKind::Component, "`component`")?;
        let (name, _) = self.ident("component name")?;
        self.expect(TokenKind::LBrace, "`{` after component name")?;
        let mut props = Vec::new();
        let mut html = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Props) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `props`")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let (prop_name, prop_span) = self.ident("component property name")?;
                    self.expect(TokenKind::Colon, "`:` after component property name")?;
                    let ty = self.type_name()?;
                    props.push(ComponentProp {
                        name: prop_name,
                        ty,
                        span: prop_span,
                    });
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace, "`}` after component properties")?;
            } else if self.at(&TokenKind::Html) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `html`")?;
                let body = match self.current().kind.clone() {
                    TokenKind::HtmlBody(body) => {
                        self.advance();
                        body
                    }
                    _ => return self.error("expected HTML body"),
                };
                self.expect(TokenKind::RBrace, "`}` after HTML body")?;
                html = Some(body);
            } else {
                return self.error("expected `props` or `html` in component definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after component definition")?;
        let Some(html) = html else {
            return self.error("component definition requires an `html` block");
        };
        Ok(ComponentDef {
            name,
            is_public,
            props,
            html,
            span: start.join(end),
        })
    }
    fn form_definition(&mut self) -> Result<FormDef, ParseError> {
        let start = self.expect(TokenKind::Form, "`form`")?;
        let (name, _) = self.ident("form name")?;
        let table = if self.at(&TokenKind::Arrow) {
            self.advance();
            Some(self.ident("table name after `->`")?.0)
        } else {
            None
        };
        self.expect(TokenKind::LBrace, "`{` after form name")?;
        let mut path = None;
        let mut post_only = false;
        let mut post_only_declared = false;
        let mut verify = None;
        let mut fields = Vec::new();
        let mut actions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at_word("route") {
                let route = self.advance();
                self.expect(TokenKind::Colon, "`:` after `route`")?;
                if path.is_some() {
                    return Err(ParseError {
                        message: "duplicate `route` property in form".into(),
                        span: route.span,
                    });
                }
                path = Some(self.string_value("form route")?);
            } else if self.at_word("post_only") {
                let marker = self.advance();
                if post_only_declared {
                    return Err(ParseError {
                        message: "duplicate `post_only` property in form".into(),
                        span: marker.span,
                    });
                }
                post_only = true;
                post_only_declared = true;
            } else if self.at_word("verify") {
                let marker = self.advance();
                if verify.is_some() {
                    return Err(ParseError {
                        message: "duplicate `verify` query in form".into(),
                        span: marker.span,
                    });
                }
                let sql_start = self.expect(TokenKind::Sql, "`sql` after form `verify`")?;
                let expression = self.sql_expression(sql_start)?;
                if !matches!(expression.kind, ExprKind::Sql { .. }) {
                    return Err(ParseError {
                        message: "form `verify` requires a SQL query".into(),
                        span: marker.span,
                    });
                }
                verify = Some(expression);
            } else if self.at(&TokenKind::Field) {
                fields.push(self.form_field()?);
            } else if self.at(&TokenKind::Fields) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `fields`")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let (field_name, field_span) = self.field_ident("form field name")?;
                    fields.push(FormField {
                        name: field_name,
                        ty: None,
                        label: None,
                        placeholder: None,
                        required: false,
                        min_length: None,
                        max: None,
                        min_value: None,
                        max_value: None,
                        choices: Vec::new(),
                        widget: None,
                        appearance: None,
                        file_upload: None,
                        readonly: false,
                        span: field_span,
                    });
                    self.skip_newlines();
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }
                self.expect(TokenKind::RBrace, "`}` after form fields")?;
            } else if self.at(&TokenKind::Action) {
                actions.push(self.form_action()?);
            } else {
                return self
                    .error("expected `route`, `post_only`, `verify`, `field`, `fields`, or `action` in form definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after form definition")?;
        Ok(FormDef {
            name,
            table,
            path,
            post_only,
            verify,
            fields,
            actions,
            span: start.join(end),
        })
    }
    fn form_field(&mut self) -> Result<FormField, ParseError> {
        let start = self.expect(TokenKind::Field, "`field`")?;
        let (name, _) = self.field_ident("form field name")?;
        let ty = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.type_name()?)
        } else {
            None
        };
        self.expect(TokenKind::LBrace, "`{` after form field")?;
        let mut field = FormField {
            name,
            ty,
            label: None,
            placeholder: None,
            required: false,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: start,
        };
        let mut max_size_seen = false;
        let mut allowed_types_seen = false;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            match self.current().kind.clone() {
                TokenKind::Label => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after label")?;
                    field.label = Some(self.string_value("label")?);
                }
                TokenKind::Placeholder => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after placeholder")?;
                    field.placeholder = Some(self.string_value("placeholder")?);
                }
                TokenKind::Required => {
                    field.required = true;
                    self.advance();
                }
                TokenKind::Max => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after max")?;
                    field.max = Some(self.positive_integer("maximum length")?);
                }
                TokenKind::Ident(ref name) if name == "min_length" => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after min_length")?;
                    field.min_length = Some(self.positive_integer("minimum length")?);
                }
                TokenKind::Ident(ref name) if name == "min_value" || name == "max_value" => {
                    let is_min = name == "min_value";
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after numeric bound")?;
                    let bound = self.string_value("numeric bound")?;
                    if is_min {
                        field.min_value = Some(bound);
                    } else {
                        field.max_value = Some(bound);
                    }
                }
                TokenKind::Ident(ref name) if name == "choices" => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after choices")?;
                    self.expect(TokenKind::LBracket, "`[` before choices")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBracket) && !self.at(&TokenKind::Eof) {
                        field.choices.push(self.string_value("choice")?);
                        self.skip_newlines();
                        if self.at(&TokenKind::Comma) {
                            self.advance();
                            self.skip_newlines();
                        } else if !self.at(&TokenKind::RBracket) {
                            return self.error("expected `,` or `]` after choice");
                        }
                    }
                    self.expect(TokenKind::RBracket, "`]` after choices")?;
                    if field.choices.is_empty() {
                        return self.error("form choices must contain at least one value");
                    }
                }
                TokenKind::Widget => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after widget")?;
                    field.widget = Some(self.form_value("widget")?);
                }
                TokenKind::Appearance => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after appearance")?;
                    field.appearance = Some(self.form_value("appearance")?);
                }
                TokenKind::Ident(ref name) if name == "max_size" => {
                    if max_size_seen {
                        return self.error("duplicate `max_size` in file field");
                    }
                    max_size_seen = true;
                    let option_span = self.current().span;
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after max_size")?;
                    let max_size = self.string_value("maximum file size, such as `10MB`")?;
                    let max_size_bytes = parse_file_size(&max_size).ok_or_else(|| ParseError {
                        message: "max_size must be a positive byte size such as `10MB` or `512KiB`"
                            .into(),
                        span: option_span,
                    })?;
                    field
                        .file_upload
                        .get_or_insert_with(|| zelyra_ast::FileUploadOptions {
                            max_size_bytes: 0,
                            allowed_types: Vec::new(),
                        })
                        .max_size_bytes = max_size_bytes;
                }
                TokenKind::Ident(ref name) if name == "allowed_types" => {
                    if allowed_types_seen {
                        return self.error("duplicate `allowed_types` in file field");
                    }
                    allowed_types_seen = true;
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after allowed_types")?;
                    self.expect(TokenKind::LBracket, "`[` before allowed_types")?;
                    self.skip_newlines();
                    let options =
                        field
                            .file_upload
                            .get_or_insert_with(|| zelyra_ast::FileUploadOptions {
                                max_size_bytes: 0,
                                allowed_types: Vec::new(),
                            });
                    while !self.at(&TokenKind::RBracket) && !self.at(&TokenKind::Eof) {
                        options
                            .allowed_types
                            .push(self.string_value("allowed MIME type")?);
                        self.skip_newlines();
                        if self.at(&TokenKind::Comma) {
                            self.advance();
                            self.skip_newlines();
                        } else if !self.at(&TokenKind::RBracket) {
                            return self.error("expected `,` or `]` after allowed MIME type");
                        }
                    }
                    self.expect(TokenKind::RBracket, "`]` after allowed_types")?;
                }
                TokenKind::Readonly => {
                    field.readonly = true;
                    self.advance();
                }
                found => return self.error(format!("unexpected form field option {found:?}")),
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after form field")?;
        field.span = start.join(end);
        Ok(field)
    }
    fn form_action(&mut self) -> Result<FormAction, ParseError> {
        let start = self.expect(TokenKind::Action, "`action`")?;
        let (name, _) = self.ident("form action name")?;
        self.expect(TokenKind::LBrace, "`{` after form action")?;
        let mut statements = Vec::new();
        let mut label = None;
        let mut icon = None;
        let mut confirm = None;
        let mut confirm_page = None;
        let mut success_page = None;
        let mut error_page = None;
        let mut fields = Vec::new();
        let mut success = None;
        let mut redirect = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        let mut capabilities = Vec::new();
        let mut execute_body = None;
        let mut guard = None;
        let mut require_match = false;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Field) {
                fields.push(self.form_field()?);
            } else if self.at(&TokenKind::Requires) {
                self.advance();
                self.expect(TokenKind::Auth, "auth after requires")?;
                requires_auth = true;
            } else if self.at(&TokenKind::Permits) {
                self.advance();
                permissions.push(self.string_value("permission")?);
            } else if self.at(&TokenKind::Uses) {
                self.advance();
                loop {
                    let (mut capability, _) = self.ident("action capability name")?;
                    if capability == "Database" && self.at(&TokenKind::LParen) {
                        self.advance();
                        let (access, _) = self.ident("database access effect")?;
                        self.expect(TokenKind::RParen, "`)` after database access effect")?;
                        capability = format!("Database({access})");
                    }
                    capabilities.push(capability);
                    self.skip_newlines();
                    if !self.at(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                    self.skip_newlines();
                }
            } else if self.at_word("require_match") {
                let marker = self.advance();
                if require_match {
                    return Err(ParseError {
                        message: "duplicate `require_match` in form action".into(),
                        span: marker.span,
                    });
                }
                require_match = true;
            } else if self.at(&TokenKind::Label) {
                self.advance();
                self.expect(TokenKind::Colon, "`:` after label")?;
                label = Some(self.string_value("action label")?);
            } else if self.at(&TokenKind::Icon) {
                self.advance();
                self.expect(TokenKind::Colon, "`:` after icon")?;
                icon = Some(self.string_value("action icon")?);
            } else if self.at(&TokenKind::Confirm) {
                self.advance();
                self.expect(TokenKind::Colon, "`:` after confirm")?;
                confirm = Some(self.string_value("confirmation message")?);
            } else if self.at(&TokenKind::ConfirmPage) {
                self.advance();
                confirm_page = Some(self.crud_confirm_view_block()?);
            } else if self.at(&TokenKind::SuccessPage) {
                self.advance();
                success_page = Some(self.crud_action_notice_block()?);
            } else if self.at(&TokenKind::ErrorPage) {
                self.advance();
                error_page = Some(self.crud_action_notice_block()?);
            } else if self.at(&TokenKind::Success) {
                self.advance();
                success = Some(self.string_value("success message")?);
            } else if self.at(&TokenKind::Redirect) {
                self.advance();
                redirect = Some(self.string_value("redirect path")?);
            } else if matches!(&self.current().kind, TokenKind::Ident(name) if name == "execute") {
                let (_, execute_span) = self.ident("`execute`")?;
                if execute_body.is_some() {
                    return Err(ParseError {
                        message: "form action may declare only one `execute` block".into(),
                        span: execute_span,
                    });
                }
                execute_body = Some(self.block()?);
            } else if matches!(&self.current().kind, TokenKind::Ident(name) if name == "guard") {
                let (_, guard_span) = self.ident("`guard`")?;
                if guard.is_some() {
                    return Err(ParseError {
                        message: "form action may declare only one `guard`".into(),
                        span: guard_span,
                    });
                }
                guard = Some(self.form_action_guard(guard_span)?);
            } else {
                statements.push(self.statement()?);
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after form action")?;
        if execute_body.is_some() && !statements.is_empty() {
            return Err(ParseError {
                message: "legacy direct statements cannot be mixed with an `execute` block".into(),
                span: start.join(end),
            });
        }
        Ok(FormAction {
            name,
            label,
            icon,
            confirm,
            confirm_page,
            success_page,
            error_page,
            fields,
            requires_auth,
            permissions,
            capabilities,
            require_match,
            statements,
            execute_body,
            guard,
            success,
            redirect,
            span: start.join(end),
        })
    }

    fn form_action_guard(
        &mut self,
        start: Span,
    ) -> Result<zelyra_ast::CrudActionGuardDef, ParseError> {
        let (when, when_span) = self.ident("`when` after action guard")?;
        if when != "when" {
            return Err(ParseError {
                message: "expected `when` after action `guard`".into(),
                span: when_span,
            });
        }
        let condition = self.expression()?;
        self.expect(TokenKind::LBrace, "`{` before action guard error")?;
        self.skip_newlines();
        let (error, error_span) = self.ident("`error` in action guard")?;
        if error != "error" {
            return Err(ParseError {
                message: "action guard requires `error: \"message\"`".into(),
                span: error_span,
            });
        }
        self.expect(TokenKind::Colon, "`:` after action guard error")?;
        let message = self.string_value("action guard error message")?;
        self.skip_newlines();
        let end = self.expect(TokenKind::RBrace, "`}` after action guard error")?;
        Ok(zelyra_ast::CrudActionGuardDef {
            condition,
            message,
            span: start.join(end),
        })
    }

    fn crud_confirm_view_block(&mut self) -> Result<CrudConfirmViewDef, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after confirm_page")?;
        let mut view = CrudConfirmViewDef::default();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (property, _) = self.ident("confirmation view property")?;
            self.expect(TokenKind::Colon, "colon after confirmation view property")?;
            match property.as_str() {
                "title" => view.title = Some(self.string_value("confirmation title")?),
                "message" => view.message = Some(self.string_value("confirmation message")?),
                "submit" => view.submit = Some(self.string_value("confirmation submit label")?),
                _ => {
                    return self.error(
                        "expected `title`, `message`, or `submit` in confirm_page definition",
                    )
                }
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after confirm_page")?;
        Ok(view)
    }

    fn crud_action_notice_block(&mut self) -> Result<CrudActionNoticeDef, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after action notice")?;
        let mut view = CrudActionNoticeDef::default();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (property, _) = self.ident("action notice property")?;
            self.expect(TokenKind::Colon, "colon after action notice property")?;
            match property.as_str() {
                "title" => view.title = Some(self.string_value("action notice title")?),
                "message" => view.message = Some(self.string_value("action notice message")?),
                _ => {
                    return self.error("expected `title` or `message` in action notice definition")
                }
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after action notice")?;
        Ok(view)
    }
    fn positive_integer(&mut self, label: &str) -> Result<u32, ParseError> {
        match self.current().kind.clone() {
            TokenKind::Int(value) if value > 0 => {
                self.advance();
                Ok(value as u32)
            }
            _ => self.error(format!("expected positive {label}")),
        }
    }
    fn form_value(&mut self, label: &str) -> Result<String, ParseError> {
        match self.current().kind.clone() {
            TokenKind::String(value) => {
                self.advance();
                Ok(value)
            }
            TokenKind::StringTemplate { value, .. } => {
                self.advance();
                Ok(value)
            }
            TokenKind::Ident(value) => {
                self.advance();
                Ok(value)
            }
            _ => self.error(format!("expected {label}")),
        }
    }
    fn table_definition(&mut self) -> Result<TableDef, ParseError> {
        let start = self.expect(TokenKind::Table, "`table`")?;
        let (name, _) = self.table_ident("table name")?;
        let tenant_scope = if self.at_word("scoped") {
            let (_, scope_start) = self.ident("`scoped`")?;
            let (by, by_span) = self.ident("`by` after `scoped`")?;
            if by != "by" {
                return Err(ParseError {
                    message: "expected `by` after `scoped`".into(),
                    span: by_span,
                });
            }
            let (column, _) = self.field_ident("tenant scope column")?;
            self.expect(TokenKind::Colon, "`:` after tenant scope column")?;
            let tenant_type = self.type_name()?;
            Some(TenantScopeDef {
                column,
                tenant_type,
                span: scope_start.join(self.tokens[self.pos - 1].span),
            })
        } else {
            None
        };
        self.expect(TokenKind::LBrace, "`{` after table name")?;
        let mut columns = Vec::new();
        let mut validations = Vec::new();
        let mut indexes = Vec::new();
        let mut uniques = Vec::new();
        let mut access = zelyra_ast::TableAccessDef::default();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Index) {
                indexes.push(self.index_definition(TokenKind::Index)?);
            } else if self.at(&TokenKind::Unique) {
                uniques.push(self.index_definition(TokenKind::Unique)?);
            } else if matches!(&self.current().kind, TokenKind::Ident(name) if name == "access") {
                if access.span.is_some() {
                    return self.error("a table may declare its access contract only once");
                }
                access = self.table_access_definition()?;
            } else if self.at_word_followed_by("validate", &TokenKind::LBrace) {
                validations.extend(self.table_validation_block()?);
            } else {
                columns.push(self.column_definition()?);
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after table definition")?;
        Ok(TableDef {
            name,
            columns,
            validations,
            indexes,
            uniques,
            access,
            tenant_scope,
            span: start.join(end),
        })
    }

    fn table_validation_block(
        &mut self,
    ) -> Result<Vec<zelyra_ast::TableValidationDef>, ParseError> {
        let (_, start) = self.ident("`validate`")?;
        self.expect(TokenKind::LBrace, "`{` after table validation")?;
        let mut validations = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let condition = self.expression()?;
            self.expect(TokenKind::Colon, "`:` before validation error message")?;
            let (message, message_span) = match &self.current().kind {
                TokenKind::String(message) => {
                    let message = message.clone();
                    let span = self.advance().span;
                    (message, span)
                }
                _ => return self.error("validation error message must be a string literal"),
            };
            validations.push(zelyra_ast::TableValidationDef {
                span: condition.span.join(message_span),
                condition,
                message,
            });
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after table validations")?;
        if validations.is_empty() {
            return Err(ParseError {
                message: "a table validation block must contain at least one rule".into(),
                span: start.join(end),
            });
        }
        Ok(validations)
    }

    fn table_access_definition(&mut self) -> Result<zelyra_ast::TableAccessDef, ParseError> {
        let (name, start) = self.ident("`access`")?;
        if name != "access" {
            return self.error("expected `access`");
        }
        self.expect(TokenKind::LBrace, "`{` after table access")?;
        let mut access = zelyra_ast::TableAccessDef {
            span: Some(start),
            ..zelyra_ast::TableAccessDef::default()
        };
        let mut seen = std::collections::HashSet::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (mode, _) = self.ident("`read`, `write`, or `read_write`")?;
            if !matches!(mode.as_str(), "read" | "write" | "read_write") {
                return self.error("table access mode must be `read`, `write`, or `read_write`");
            }
            if !seen.insert(mode.clone()) {
                return self.error(format!(
                    "table access mode `{mode}` is declared more than once"
                ));
            }
            self.expect(TokenKind::Colon, "`:` after table access mode")?;
            let modules = self.table_access_module_list()?;
            match mode.as_str() {
                "read" => access.read = modules,
                "write" => access.write = modules,
                "read_write" => access.read_write = modules,
                _ => unreachable!(),
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after table access contract")?;
        Ok(access)
    }

    fn table_access_module_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(TokenKind::LBracket, "`[` before granted module paths")?;
        let mut modules = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBracket) && !self.at(&TokenKind::Eof) {
            let path = match self.current().kind.clone() {
                TokenKind::String(path) => {
                    self.advance();
                    path
                }
                TokenKind::StringTemplate { value: path, .. } => {
                    self.advance();
                    path
                }
                _ => {
                    return self.error("table access grants must be project-relative path strings")
                }
            };
            if modules.contains(&path) {
                return self.error(format!("table access grant `{path}` is repeated"));
            }
            modules.push(path);
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else if !self.at(&TokenKind::RBracket) {
                return self.error("expected `,` between table access grants");
            }
        }
        self.expect(TokenKind::RBracket, "`]` after granted module paths")?;
        Ok(modules)
    }
    fn column_definition(&mut self) -> Result<ColumnDef, ParseError> {
        let (name, span) = self.field_ident("column name")?;
        self.expect(TokenKind::Colon, "`:` after column name")?;
        let (ty, length, decimal_precision) = self.column_type()?;
        let mut storage_name = None;
        let mut auto_sequence = None;
        let mut computed = None;
        let mut required = false;
        let mut primary_key = false;
        let mut auto = false;
        let mut unique = false;
        let mut unique_within_tenant = false;
        let mut default = None;
        let mut references = None;
        let mut on_delete = None;
        loop {
            match self.current().kind.clone() {
                TokenKind::Required => {
                    required = true;
                    self.advance();
                }
                TokenKind::Primary => {
                    primary_key = true;
                    self.advance();
                }
                TokenKind::Auto => {
                    auto = true;
                    self.advance();
                }
                TokenKind::Unique => {
                    self.advance();
                    if self.at_word("within") {
                        self.advance();
                        let (scope, scope_span) = self.ident("`tenant` after `unique within`")?;
                        if scope != "tenant" {
                            return Err(ParseError {
                                message: "expected `tenant` after `unique within`".into(),
                                span: scope_span,
                            });
                        }
                        if unique_within_tenant {
                            return self.error("`unique within tenant` may be specified only once");
                        }
                        unique_within_tenant = true;
                    } else {
                        if unique {
                            return self.error("`unique` may be specified only once");
                        }
                        unique = true;
                    }
                }
                TokenKind::Default => {
                    self.advance();
                    default = Some(self.default_value()?);
                }
                TokenKind::Ident(ref value) if value == "references" => {
                    self.advance();
                    if references.is_some() {
                        return self.error("`references` may be specified only once");
                    }
                    references = Some(self.ident("referenced table name")?.0);
                }
                TokenKind::Ident(ref value) if value == "on_delete" => {
                    self.advance();
                    if on_delete.is_some() {
                        return self.error("`on_delete` may be specified only once");
                    }
                    let (action, _) = self.ident("referential action")?;
                    on_delete = Some(match action.as_str() {
                        "no_action" => ReferentialAction::NoAction,
                        "restrict" => ReferentialAction::Restrict,
                        "cascade" => ReferentialAction::Cascade,
                        "set_null" => ReferentialAction::SetNull,
                        "set_default" => ReferentialAction::SetDefault,
                        _ => return self.error(format!("unknown on-delete action `{action}`")),
                    });
                }
                TokenKind::Ident(ref value) if value == "storage_name" => {
                    self.advance();
                    if storage_name.is_some() {
                        return self.error("`storage_name` may be specified only once");
                    }
                    storage_name = Some(self.string_value("quoted database storage name")?);
                }
                TokenKind::Ident(ref value) if value == "auto_sequence" => {
                    self.advance();
                    if auto_sequence.is_some() {
                        return self.error("`auto_sequence` may be specified only once");
                    }
                    self.expect(TokenKind::LParen, "`(` after `auto_sequence`")?;
                    auto_sequence = Some(self.string_value("quoted number-series format")?);
                    self.expect(TokenKind::RParen, "`)` after number-series format")?;
                }
                TokenKind::Ident(ref value) if value == "compute" => {
                    self.advance();
                    if computed.is_some() {
                        return self.error("`compute` may be specified only once");
                    }
                    self.expect(TokenKind::LParen, "`(` after `compute`")?;
                    computed = Some(self.expression()?);
                    self.expect(TokenKind::RParen, "`)` after computed expression")?;
                }
                _ => break,
            }
        }
        Ok(ColumnDef {
            name,
            ty,
            length,
            storage_name,
            decimal_precision,
            auto_sequence,
            computed,
            required,
            primary_key,
            auto,
            unique,
            unique_within_tenant,
            default,
            references,
            on_delete,
            span,
        })
    }
    fn column_type(&mut self) -> Result<ParsedColumnType, ParseError> {
        let ty = self.type_name()?;
        if self.at(&TokenKind::LParen) {
            self.advance();
            let first = match self.current().kind.clone() {
                TokenKind::Int(value) if value > 0 && value <= 65_535 => {
                    self.advance();
                    value as u32
                }
                _ => return self.error("expected a positive column type size"),
            };
            if ty == Type::String {
                let length = first;
                if !self.at(&TokenKind::RParen) {
                    return self.error("String columns accept only one length value");
                }
                self.expect(TokenKind::RParen, "`)` after String length")?;
                let ty = if self.at(&TokenKind::Question) {
                    self.advance();
                    Type::Option(Box::new(ty))
                } else {
                    ty
                };
                return Ok((ty, Some(length), None));
            }
            if ty == Type::Decimal {
                self.expect(TokenKind::Comma, "`,` between Decimal precision and scale")?;
                let scale = match self.current().kind.clone() {
                    TokenKind::Int(value) if (0..=9).contains(&value) => {
                        self.advance();
                        value as u32
                    }
                    _ => return self.error("Decimal scale must be an integer from 0 to 9"),
                };
                self.expect(TokenKind::RParen, "`)` after Decimal precision and scale")?;
                if first > 39 || scale >= first {
                    return self
                        .error("Decimal precision must be 1 to 39 and greater than its scale");
                }
                let ty = if self.at(&TokenKind::Question) {
                    self.advance();
                    Type::Option(Box::new(ty))
                } else {
                    ty
                };
                return Ok((ty, None, Some((first, scale))));
            }
            self.error("parenthesized sizes are supported only for String and Decimal columns")
        } else {
            Ok((ty, None, None))
        }
    }
    fn default_value(&mut self) -> Result<DefaultValue, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Int(value) => Ok(DefaultValue::Int(value)),
            TokenKind::True => Ok(DefaultValue::Bool(true)),
            TokenKind::False => Ok(DefaultValue::Bool(false)),
            TokenKind::String(value) => Ok(DefaultValue::String(value)),
            TokenKind::StringTemplate { value, .. } => Ok(DefaultValue::String(value)),
            TokenKind::Ident(value) => Ok(DefaultValue::Ident(value)),
            found => self.error(format!("expected default value, found {found:?}")),
        }
    }
    fn index_definition(&mut self, kind: TokenKind) -> Result<IndexDef, ParseError> {
        let start = self.expect(kind, "index declaration")?;
        self.expect(TokenKind::LBrace, "`{` after index declaration")?;
        let mut columns = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (column, _) = self.field_ident("indexed column name")?;
            columns.push(column);
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after index columns")?;
        if columns.is_empty() {
            return self.error("index must contain at least one column");
        }
        Ok(IndexDef {
            columns,
            span: start.join(end),
        })
    }
    fn type_definition(&mut self, is_public: bool) -> Result<TypeDef, ParseError> {
        let start = self.expect(TokenKind::Type, "`type`")?;
        let (name, _) = self.ident("type name")?;
        self.expect(TokenKind::Equal, "`=` in type definition")?;
        let target = self.type_name()?;
        Ok(TypeDef {
            name,
            is_public,
            target,
            span: start,
        })
    }

    fn record_definition(&mut self, is_public: bool) -> Result<RecordDef, ParseError> {
        let (start, is_class) = if self.at_word("class") {
            (self.advance().span, true)
        } else {
            (self.expect(TokenKind::Struct, "`struct`")?, false)
        };
        let (name, _) = self.ident("record name")?;
        let type_params = self.type_parameter_names()?;
        self.expect(TokenKind::LBrace, "`{` after record name")?;
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (field_name, span) = self.field_ident("record field name")?;
            self.expect(TokenKind::Colon, "`:` after record field name")?;
            let ty = self.type_name()?;
            fields.push(RecordField {
                name: field_name,
                ty,
                span,
            });
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after record definition")?;
        if fields.is_empty() {
            return self.error("record must contain at least one field");
        }
        Ok(RecordDef {
            name,
            is_public,
            is_class,
            type_params,
            fields,
            span: start.join(end),
        })
    }

    fn enum_definition(&mut self, is_public: bool) -> Result<zelyra_ast::EnumDef, ParseError> {
        let start = self.expect(TokenKind::Enum, "`enum`")?;
        let (name, _) = self.ident("enum name")?;
        let type_params = self.type_parameter_names()?;
        self.expect(TokenKind::LBrace, "`{` after enum name")?;
        let mut variants = Vec::new();
        let mut variant_payloads = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (variant, span) = self.ident("enum variant")?;
            if !seen.insert(variant.clone()) {
                return Err(ParseError {
                    message: format!("enum variant `{variant}` is declared more than once"),
                    span,
                });
            }
            let mut payload_types = Vec::new();
            if self.at(&TokenKind::LParen) {
                self.advance();
                self.skip_newlines();
                if !self.at(&TokenKind::RParen) {
                    loop {
                        payload_types.push(self.type_name()?);
                        self.skip_newlines();
                        if !self.at(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                        self.skip_newlines();
                    }
                }
                let end = self.expect(TokenKind::RParen, "`)` after enum payload types")?;
                if payload_types.is_empty() {
                    return Err(ParseError {
                        message: "unit enum variants must not use empty parentheses; write the variant name without `()`".into(),
                        span: span.join(end),
                    });
                }
            }
            variants.push(variant);
            variant_payloads.push((
                variants.last().expect("variant was just pushed").clone(),
                payload_types,
            ));
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after enum definition")?;
        if variants.is_empty() {
            return Err(ParseError {
                message: "enum must contain at least one unit variant".into(),
                span: start.join(end),
            });
        }
        Ok(zelyra_ast::EnumDef {
            name,
            is_public,
            type_params,
            variants,
            variant_payloads,
            span: start.join(end),
        })
    }
    fn type_parameter_names(&mut self) -> Result<Vec<String>, ParseError> {
        if !self.at(&TokenKind::Less) {
            return Ok(Vec::new());
        }
        self.advance();
        let mut parameters = Vec::new();
        let mut seen = std::collections::HashSet::new();
        loop {
            let (parameter, span) = self.ident("generic type parameter")?;
            if !seen.insert(parameter.clone()) {
                return Err(ParseError {
                    message: format!(
                        "generic type parameter `{parameter}` is declared more than once"
                    ),
                    span,
                });
            }
            parameters.push(parameter);
            if !self.at(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }
        self.expect(TokenKind::Greater, "`>` after generic type parameters")?;
        Ok(parameters)
    }

    fn function(&mut self, is_public: bool) -> Result<Function, ParseError> {
        let start = self.expect(TokenKind::Fn, "`fn`")?;
        let (name, _) = self.ident("function name")?;
        let mut type_params = Vec::new();
        if self.at(&TokenKind::Less) {
            self.advance();
            let mut seen = std::collections::HashSet::new();
            loop {
                let (parameter, span) = self.ident("generic type parameter")?;
                if !seen.insert(parameter.clone()) {
                    return Err(ParseError {
                        message: format!(
                            "generic type parameter `{parameter}` is declared more than once"
                        ),
                        span,
                    });
                }
                type_params.push(parameter);
                if !self.at(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect(TokenKind::Greater, "`>` after generic type parameters")?;
        }
        self.expect(TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();
        self.skip_newlines();
        if !self.at(&TokenKind::RParen) {
            loop {
                let (param_name, param_span) = self.ident("parameter name")?;
                self.expect(TokenKind::Colon, "`:` after parameter name")?;
                let ty = self.type_name()?;
                params.push(Param {
                    name: param_name,
                    ty,
                    span: param_span,
                });
                self.skip_newlines();
                if !self.at(&TokenKind::Comma) {
                    break;
                }
                self.advance();
                self.skip_newlines();
            }
        }
        self.expect(TokenKind::RParen, "`)`")?;
        let return_type = if self.at(&TokenKind::Arrow) {
            self.advance();
            Some(self.type_name()?)
        } else {
            None
        };
        self.skip_newlines();
        let mut capabilities = Vec::new();
        if self.at(&TokenKind::Uses) {
            self.advance();
            loop {
                let (mut capability, _) = self.ident("capability name")?;
                if capability == "Database" && self.at(&TokenKind::LParen) {
                    self.advance();
                    let (access, _) = self.ident("database access effect")?;
                    self.expect(TokenKind::RParen, "`)` after database access effect")?;
                    capability = format!("Database({access})");
                }
                capabilities.push(capability);
                self.skip_newlines();
                if !self.at(&TokenKind::Comma) {
                    break;
                }
                self.advance();
                self.skip_newlines();
            }
        }
        self.skip_newlines();
        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        loop {
            if self.at(&TokenKind::Requires) {
                self.advance();
                requires.extend(self.contract_block("requires")?);
            } else if self.at(&TokenKind::Ensures) {
                self.advance();
                ensures.extend(self.contract_block("ensures")?);
            } else {
                break;
            }
            self.skip_newlines();
        }
        let body = self.block()?;
        Ok(Function {
            name,
            is_public,
            type_params,
            params,
            return_type,
            capabilities,
            requires,
            ensures,
            span: start.join(body.span),
            body,
        })
    }

    fn contract_block(&mut self, kind: &str) -> Result<Vec<Expr>, ParseError> {
        let opening_label = format!("`{{` after `{kind}`");
        self.expect(TokenKind::LBrace, &opening_label)?;
        let mut expressions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            expressions.push(self.expression()?);
            self.skip_newlines();
        }
        let closing_label = format!("`}}` after `{kind}`");
        self.expect(TokenKind::RBrace, &closing_label)?;
        Ok(expressions)
    }
    fn type_name(&mut self) -> Result<Type, ParseError> {
        self.with_nesting(Self::type_name_inner)
    }

    fn looks_like_generic_call(&self) -> bool {
        if !self.at(&TokenKind::Less) {
            return false;
        }
        let mut depth = 0_usize;
        for (offset, token) in self.tokens[self.pos..].iter().enumerate() {
            match &token.kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self
                            .tokens
                            .get(self.pos + offset + 1)
                            .is_some_and(|next| next.kind == TokenKind::LParen);
                    }
                }
                TokenKind::Eof | TokenKind::Newline => return false,
                _ => {}
            }
        }
        false
    }

    fn type_name_inner(&mut self) -> Result<Type, ParseError> {
        let (mut name, _) = self.ident("type name")?;
        while self.at(&TokenKind::DoubleColon) {
            self.advance();
            let (segment, _) = self.ident("type name after `::`")?;
            name.push_str("::");
            name.push_str(&segment);
        }
        let mut ty = match name.as_str() {
            "Int" => Type::Int,
            "UInt" => Type::UInt,
            "Float" => Type::Float,
            "Decimal" => Type::Decimal,
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Char" => Type::Char,
            "Bytes" => Type::Bytes,
            "Json" => Type::Json,
            "Timestamp" => Type::Timestamp,
            "Date" => Type::Date,
            "Time" => Type::Time,
            "Duration" => Type::Duration,
            "Unit" => Type::Unit,
            "Option" => {
                self.expect(TokenKind::Less, "`<` after `Option`")?;
                let inner = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after Option type")?;
                Type::Option(Box::new(inner))
            }
            "Result" => {
                self.expect(TokenKind::Less, "`<` after `Result`")?;
                let ok = self.type_name()?;
                self.expect(TokenKind::Comma, "`,` between Result types")?;
                let error = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after Result types")?;
                Type::Result(Box::new(ok), Box::new(error))
            }
            "Map" => {
                self.expect(TokenKind::Less, "`<` after `Map`")?;
                let key = self.type_name()?;
                self.expect(TokenKind::Comma, "`,` between Map key and value types")?;
                let value = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after Map value type")?;
                Type::Map(Box::new(key), Box::new(value))
            }
            "List" => {
                self.expect(TokenKind::Less, "`<` after `List`")?;
                let inner = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after List element type")?;
                Type::Array(Box::new(inner))
            }
            "Set" => {
                self.expect(TokenKind::Less, "`<` after `Set`")?;
                let inner = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after Set element type")?;
                Type::Set(Box::new(inner))
            }
            "Tuple" => {
                self.expect(TokenKind::Less, "`<` after `Tuple`")?;
                let mut items = vec![self.type_name()?];
                while self.at(&TokenKind::Comma) {
                    self.advance();
                    items.push(self.type_name()?);
                }
                self.expect(TokenKind::Greater, "`>` after Tuple item types")?;
                if items.len() < 2 {
                    return self.error("Tuple requires at least two item types");
                }
                Type::Tuple(items)
            }
            "HttpResult" => {
                self.expect(TokenKind::Less, "`<` after `HttpResult`")?;
                let response = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after HttpResult type")?;
                Type::HttpResult(Box::new(response))
            }
            "Function" => {
                self.expect(TokenKind::Less, "`<` after `Function`")?;
                self.expect(TokenKind::LParen, "`(` before function parameter types")?;
                let mut params = Vec::new();
                if !self.at(&TokenKind::RParen) {
                    loop {
                        params.push(self.type_name()?);
                        if !self.at(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen, "`)` after function parameter types")?;
                self.expect(TokenKind::Arrow, "`->` before function return type")?;
                let return_type = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after function type")?;
                let mut capabilities = Vec::new();
                if self.at(&TokenKind::Uses) {
                    self.advance();
                    self.expect(TokenKind::LBrace, "`{` after function-type `uses`")?;
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let capability = match &self.current().kind {
                            TokenKind::Ident(name) => name.clone(),
                            TokenKind::Database => "Database".into(),
                            _ => return self.error("expected a capability name in function type"),
                        };
                        let span = self.advance().span;
                        if capabilities.contains(&capability) {
                            return Err(ParseError {
                                message: format!(
                                    "capability `{capability}` is declared more than once"
                                ),
                                span,
                            });
                        }
                        capabilities.push(capability);
                        self.skip_newlines();
                        if self.at(&TokenKind::Comma) {
                            self.advance();
                            self.skip_newlines();
                        } else if !self.at(&TokenKind::RBrace) {
                            return self.error("expected `,` or `}` in function-type capabilities");
                        }
                    }
                    self.expect(TokenKind::RBrace, "`}` after function-type capabilities")?;
                }
                Type::Function {
                    params,
                    return_type: Box::new(return_type),
                    capabilities,
                }
            }
            _ if self.at(&TokenKind::Less) => {
                self.advance();
                let mut args = Vec::new();
                loop {
                    args.push(self.type_name()?);
                    if !self.at(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.expect(TokenKind::Greater, "`>` after generic type arguments")?;
                Type::Applied { name, args }
            }
            _ => Type::Named(name),
        };
        if self.at(&TokenKind::Question) {
            self.advance();
            ty = Type::Option(Box::new(ty));
        }
        while self.at(&TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket, "`]` after array type")?;
            ty = Type::Array(Box::new(ty));
        }
        Ok(ty)
    }
    fn block(&mut self) -> Result<Block, ParseError> {
        self.with_nesting(Self::block_inner)
    }

    fn block_inner(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LBrace, "`{`")?;
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            statements.push(self.statement()?);
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}`")?;
        Ok(Block {
            statements,
            span: start.join(end),
        })
    }
    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.looks_like_tuple_assignment() {
            let start = self.expect(TokenKind::LParen, "`(` before tuple assignment")?;
            let mut names = Vec::new();
            let mut seen = HashSet::new();
            loop {
                let (name, span) = self.ident("tuple assignment variable")?;
                if !seen.insert(name.clone()) {
                    return Err(ParseError {
                        message: format!("duplicate variable `{name}` in tuple assignment"),
                        span,
                    });
                }
                names.push(name);
                if !self.at(&TokenKind::Comma) {
                    break;
                }
                self.advance();
                if self.at(&TokenKind::RParen) {
                    break;
                }
            }
            let close = self.expect(TokenKind::RParen, "`)` after tuple assignment names")?;
            if names.len() < 2 {
                return Err(ParseError {
                    message: "tuple assignment requires at least two variables".into(),
                    span: start.join(close),
                });
            }
            self.expect(TokenKind::Equal, "`=` after tuple assignment names")?;
            let value = self.expression()?;
            let span = start.join(value.span);
            return Ok(Stmt::TupleBindOrAssign { names, value, span });
        }
        if self.at(&TokenKind::Return) {
            let span = self.advance().span;
            let value = if self.at(&TokenKind::Newline)
                || self.at(&TokenKind::Semicolon)
                || self.at(&TokenKind::RBrace)
            {
                None
            } else {
                Some(self.expression()?)
            };
            return Ok(Stmt::Return { value, span });
        }
        if self.at(&TokenKind::If) {
            return self.if_statement(false);
        }
        if self.at(&TokenKind::While) {
            let start = self.advance().span;
            let condition = self.expression()?;
            let mut invariants = Vec::new();
            while self.at(&TokenKind::Invariant) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `invariant`")?;
                invariants.push(self.expression()?);
                self.expect(TokenKind::RBrace, "`}` after loop invariant")?;
                self.skip_newlines();
            }
            let body = self.block()?;
            return Ok(Stmt::While {
                condition,
                invariants,
                body: body.clone(),
                span: start.join(body.span),
            });
        }
        if self.at(&TokenKind::For) {
            let start = self.advance().span;
            let (name, additional_names) = if self.at(&TokenKind::LParen) {
                self.advance();
                let mut names = Vec::new();
                let mut seen = HashSet::new();
                loop {
                    let (name, name_span) = self.ident("loop variable name")?;
                    if !seen.insert(name.clone()) {
                        return Err(ParseError {
                            message: format!("duplicate loop variable `{name}` in tuple pattern"),
                            span: name_span,
                        });
                    }
                    names.push(name);
                    if !self.at(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.expect(TokenKind::RParen, "`)` after tuple loop variables")?;
                if names.len() < 2 {
                    return Err(ParseError {
                        message: "tuple loop patterns need at least two variables".into(),
                        span: start,
                    });
                }
                let first = names.remove(0);
                (first, names)
            } else {
                let (name, _) = self.ident("loop variable name")?;
                (name, Vec::new())
            };
            self.expect(TokenKind::In, "`in` after loop variable")?;
            let mut iterable = self.expression()?;
            if self.at_word("step") {
                let step_keyword = self.advance().span;
                let is_integer_range = matches!(
                    &iterable.kind,
                    ExprKind::Call {
                        name,
                        type_args,
                        args,
                    } if name == "range" && type_args.is_empty() && args.len() == 2
                );
                if !is_integer_range {
                    return Err(ParseError {
                        message: "`step` is only supported for integer ranges".into(),
                        span: step_keyword,
                    });
                }
                let step = self.expression()?;
                if let ExprKind::Call { args, .. } = &mut iterable.kind {
                    args.push(step);
                    iterable.span = iterable
                        .span
                        .join(args.last().expect("step was added").span);
                }
            }
            let body = self.block()?;
            return Ok(Stmt::For {
                name,
                additional_names,
                iterable,
                body: body.clone(),
                span: start.join(body.span),
            });
        }
        if self.at(&TokenKind::Loop) {
            let start = self.advance().span;
            self.skip_newlines();
            let mut invariants = Vec::new();
            while self.at(&TokenKind::Invariant) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `invariant`")?;
                invariants.push(self.expression()?);
                self.expect(TokenKind::RBrace, "`}` after loop invariant")?;
                self.skip_newlines();
            }
            let body = self.block()?;
            return Ok(Stmt::Loop {
                invariants,
                body: body.clone(),
                span: start.join(body.span),
            });
        }
        if self.at(&TokenKind::Break) {
            let span = self.advance().span;
            return Ok(Stmt::Break { span });
        }
        if self.at(&TokenKind::Continue) {
            let span = self.advance().span;
            return Ok(Stmt::Continue { span });
        }
        if self.at(&TokenKind::Match) {
            let start = self.advance().span;
            let value = self.expression()?;
            self.expect(TokenKind::LBrace, "`{` after match expression")?;
            let mut arms = Vec::new();
            self.skip_newlines();
            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                let pattern = self.pattern()?;
                let guard = if self.at(&TokenKind::If) {
                    self.advance();
                    Some(self.expression()?)
                } else {
                    None
                };
                self.expect(TokenKind::FatArrow, "`=>` after match pattern")?;
                let body = self.block()?;
                let span = pattern.span.join(body.span);
                arms.push(MatchArm {
                    pattern,
                    guard,
                    body,
                    span,
                });
                self.skip_newlines();
            }
            let end = self.expect(TokenKind::RBrace, "`}` after match arms")?;
            return Ok(Stmt::Match {
                value,
                arms,
                span: start.join(end),
            });
        }
        if self.at(&TokenKind::Transaction) {
            let start = self.advance().span;
            let body = self.block()?;
            return Ok(Stmt::Transaction {
                span: start.join(body.span),
                body,
            });
        }
        if self.at(&TokenKind::Parallel) {
            let start = self.advance().span;
            let body = self.block()?;
            return Ok(Stmt::Parallel {
                span: start.join(body.span),
                body,
            });
        }
        if self.at(&TokenKind::Mutable) {
            let start = self.advance().span;
            let (name, _) = self.ident("binding name")?;
            let ty = if self.at(&TokenKind::Colon) {
                self.advance();
                Some(self.type_name()?)
            } else {
                None
            };
            self.expect(TokenKind::Equal, "`=` in binding")?;
            let value = self.expression()?;
            return Ok(Stmt::Let {
                name,
                ty,
                value,
                mutable: true,
                span: start,
            });
        }
        if let TokenKind::Ident(name) = &self.current().kind {
            let name = name.clone();
            let start = self.current().span;
            if self.looks_like_array_index_assignment() {
                self.advance();
                self.expect(TokenKind::LBracket, "`[` after mutable array name")?;
                let index = self.expression()?;
                self.expect(TokenKind::RBracket, "`]` after mutable array index")?;
                let operation = if self.at(&TokenKind::Equal) {
                    self.advance();
                    ArrayIndexAssignOp::Set
                } else {
                    self.expect(TokenKind::PlusEqual, "`+=` after mutable array index")?;
                    ArrayIndexAssignOp::Add
                };
                let value = self.expression()?;
                let span = start.join(value.span);
                return Ok(Stmt::ArrayIndexAssign {
                    name,
                    index,
                    value,
                    operation,
                    span,
                });
            }
            if self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.kind == TokenKind::Colon)
            {
                self.advance();
                self.advance();
                let ty = self.type_name()?;
                self.expect(TokenKind::Equal, "`=` in binding")?;
                let value = self.expression()?;
                return Ok(Stmt::Let {
                    name,
                    ty: Some(ty),
                    value,
                    mutable: false,
                    span: start,
                });
            }
            if self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.kind == TokenKind::Equal)
            {
                self.advance();
                self.advance();
                let value = self.expression()?;
                return Ok(Stmt::BindOrAssign {
                    name,
                    value,
                    span: start,
                });
            }
        }
        let expression = self.expression()?;
        if self.at(&TokenKind::Equal) {
            self.advance();
            let value = self.expression()?;
            let span = expression.span.join(value.span);
            let ExprKind::Field { target, field } = expression.kind else {
                return self
                    .error("assignment target must be a class field such as `customer.name`");
            };
            let field_span = target.span.join(expression.span);
            return Ok(Stmt::Expr(Expr {
                kind: ExprKind::Call {
                    name: "object_set_field".into(),
                    type_args: Vec::new(),
                    args: vec![
                        *target,
                        Expr {
                            kind: ExprKind::String(field),
                            span: field_span,
                        },
                        value,
                    ],
                },
                span,
            }));
        }
        Ok(Stmt::Expr(expression))
    }

    fn looks_like_tuple_assignment(&self) -> bool {
        if !self.at(&TokenKind::LParen) {
            return false;
        }
        let mut position = self.pos + 1;
        let mut names = 0;
        loop {
            if !matches!(
                self.tokens.get(position).map(|token| &token.kind),
                Some(TokenKind::Ident(_))
            ) {
                return false;
            }
            names += 1;
            position += 1;
            match self.tokens.get(position).map(|token| &token.kind) {
                Some(TokenKind::Comma) => position += 1,
                Some(TokenKind::RParen) => {
                    return names >= 2
                        && self
                            .tokens
                            .get(position + 1)
                            .is_some_and(|token| token.kind == TokenKind::Equal);
                }
                _ => return false,
            }
        }
    }

    fn looks_like_array_index_assignment(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Ident(_))
            || !self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.kind == TokenKind::LBracket)
        {
            return false;
        }
        let mut position = self.pos + 1;
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(position) {
            match token.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(position + 1).map(|token| &token.kind),
                            Some(TokenKind::Equal)
                        ) || matches!(
                            self.tokens.get(position + 1).map(|token| &token.kind),
                            Some(TokenKind::PlusEqual)
                        );
                    }
                }
                TokenKind::Eof | TokenKind::Newline => return false,
                _ => {}
            }
            position += 1;
        }
        false
    }

    fn if_statement(&mut self, is_else_if: bool) -> Result<Stmt, ParseError> {
        let start = self.advance().span;
        let condition = self.expression()?;
        let then_block = self.block().map_err(|mut error| {
            if is_else_if && error.message.starts_with("expected `{`") {
                error.message =
                    "expected a block after `else if`; use `else if condition { ... }`".into();
            }
            error
        })?;
        self.skip_newlines();
        let else_block = if self.at(&TokenKind::Else) {
            self.advance();
            self.skip_newlines();
            if self.at(&TokenKind::If) {
                let if_stmt = self.if_statement(true)?;
                let span = if_stmt.span();
                Some(Block {
                    statements: vec![if_stmt],
                    span,
                })
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        let end = else_block.as_ref().map_or(then_block.span, |b| b.span);
        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
            span: start.join(end),
        })
    }

    fn pattern(&mut self) -> Result<Pattern, ParseError> {
        self.with_nesting(Self::pattern_inner)
    }

    fn pattern_inner(&mut self) -> Result<Pattern, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) if name == "_" => Ok(Pattern {
                kind: PatternKind::Wildcard,
                span: token.span,
            }),
            TokenKind::Ident(name) => {
                if self.at(&TokenKind::DoubleColon) {
                    let mut path = name;
                    let mut end = token.span;
                    while self.at(&TokenKind::DoubleColon) {
                        self.advance();
                        let (segment, segment_span) = self.ident("enum path segment after `::`")?;
                        path.push_str("::");
                        path.push_str(&segment);
                        end = segment_span;
                    }
                    let Some((enum_name, variant)) = path.rsplit_once("::") else {
                        unreachable!("the path has at least two segments")
                    };
                    let enum_name = enum_name.to_owned();
                    if self.at(&TokenKind::LParen) {
                        self.advance();
                        let mut payloads = Vec::new();
                        self.skip_newlines();
                        if !self.at(&TokenKind::RParen) {
                            loop {
                                payloads.push(self.pattern()?);
                                self.skip_newlines();
                                if !self.at(&TokenKind::Comma) {
                                    break;
                                }
                                self.advance();
                                self.skip_newlines();
                            }
                        }
                        let close =
                            self.expect(TokenKind::RParen, "`)` after enum payload pattern")?;
                        return Ok(Pattern {
                            kind: PatternKind::EnumVariantPayload {
                                enum_name,
                                variant: variant.to_owned(),
                                payloads,
                            },
                            span: token.span.join(close),
                        });
                    }
                    return Ok(Pattern {
                        kind: PatternKind::EnumVariant {
                            enum_name,
                            variant: variant.to_owned(),
                        },
                        span: token.span.join(end),
                    });
                }
                if self.at(&TokenKind::LParen) {
                    self.advance();
                    let inner = if self.at(&TokenKind::RParen) {
                        None
                    } else {
                        Some(Box::new(self.pattern()?))
                    };
                    let end = self.expect(TokenKind::RParen, "`)` after pattern")?;
                    Ok(Pattern {
                        kind: PatternKind::Constructor { name, inner },
                        span: token.span.join(end),
                    })
                } else if self.at(&TokenKind::LBrace) {
                    self.advance();
                    let mut fields = Vec::new();
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let (field, field_span) = self.field_ident("record pattern field")?;
                        let value = if self.at(&TokenKind::Colon) {
                            self.advance();
                            self.pattern()?
                        } else {
                            Pattern {
                                kind: PatternKind::Variable(field.clone()),
                                span: field_span,
                            }
                        };
                        fields.push((field, value));
                        self.skip_newlines();
                        if self.at(&TokenKind::Comma) {
                            self.advance();
                            self.skip_newlines();
                        } else if !self.at(&TokenKind::RBrace) {
                            return self.error("expected `,` or `}` in record pattern");
                        }
                    }
                    let end = self.expect(TokenKind::RBrace, "`}` after record pattern")?;
                    Ok(Pattern {
                        kind: PatternKind::Record {
                            type_name: name,
                            fields,
                        },
                        span: token.span.join(end),
                    })
                } else {
                    Ok(Pattern {
                        kind: PatternKind::Variable(name),
                        span: token.span,
                    })
                }
            }
            TokenKind::Int(value) => Ok(Pattern {
                kind: PatternKind::Int(value),
                span: token.span,
            }),
            TokenKind::True => Ok(Pattern {
                kind: PatternKind::Bool(true),
                span: token.span,
            }),
            TokenKind::False => Ok(Pattern {
                kind: PatternKind::Bool(false),
                span: token.span,
            }),
            TokenKind::String(value) => Ok(Pattern {
                kind: PatternKind::String(value),
                span: token.span,
            }),
            TokenKind::StringTemplate { value, .. } => Ok(Pattern {
                kind: PatternKind::String(value),
                span: token.span,
            }),
            TokenKind::Char(value) => Ok(Pattern {
                kind: PatternKind::Char(value),
                span: token.span,
            }),
            found => self.error(format!("expected pattern, found {found:?}")),
        }
    }
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.with_nesting(|parser| parser.binary(0))
    }

    fn interpolated_string(&self, value: &str, raw: &str, span: Span) -> Result<Expr, ParseError> {
        let source_offsets = decoded_string_source_offsets(raw, value, span.start + 1);
        let mut parts = Vec::new();
        let mut cursor = 0;
        let mut literal_start = 0;
        let mut has_interpolation = false;

        while cursor < value.len() {
            if value[cursor..].starts_with("{{") || value[cursor..].starts_with("}}") {
                cursor += 2;
                continue;
            }
            if !value[cursor..].starts_with('{') {
                cursor += value[cursor..].chars().next().unwrap().len_utf8();
                continue;
            }
            let Some(end) = interpolation_end(value, cursor + 1) else {
                cursor += 1;
                continue;
            };

            push_interpolation_text(&mut parts, &value[literal_start..cursor], span);
            let fragment = &value[cursor + 1..end];
            let leading_whitespace = fragment.len() - fragment.trim_start().len();
            let source = fragment.trim();
            if source.is_empty() {
                return Ok(template_literal(value, span));
            }
            let mut tokens = match lex(source) {
                Ok(tokens) => tokens,
                Err(_) => return Ok(template_literal(value, span)),
            };
            // Map decoded expression offsets back through string escapes so
            // diagnostics and semantic tools use the original source spans.
            for token in &mut tokens {
                if let Some(offsets) = &source_offsets {
                    let start = cursor + 1 + leading_whitespace + token.span.start;
                    let end = cursor + 1 + leading_whitespace + token.span.end;
                    if let (Some(source_start), Some(source_end)) =
                        (offsets.get(start), offsets.get(end))
                    {
                        token.span =
                            interpolation_source_span(*source_start, *source_end, raw, span);
                        continue;
                    }
                }
                token.span = span;
            }
            let mut parser = Parser {
                tokens: &tokens,
                pos: 0,
                nesting_depth: self.nesting_depth,
            };
            let expression = match parser.expression() {
                Ok(expression) => expression,
                Err(_) => return Ok(template_literal(value, span)),
            };
            parser.skip_newlines();
            if !parser.at(&TokenKind::Eof) {
                return Ok(template_literal(value, span));
            }
            let expression_span = expression.span;
            parts.push(Expr {
                kind: ExprKind::Call {
                    name: "to_string".into(),
                    type_args: Vec::new(),
                    args: vec![expression],
                },
                span: expression_span,
            });
            has_interpolation = true;
            cursor = end + 1;
            literal_start = cursor;
        }
        push_interpolation_text(&mut parts, &value[literal_start..], span);

        if !has_interpolation {
            return Ok(template_literal(value, span));
        }
        let mut parts = parts.into_iter();
        let Some(mut expression) = parts.next() else {
            return Ok(Expr {
                kind: ExprKind::String(String::new()),
                span,
            });
        };
        for part in parts {
            expression = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expression),
                    op: BinaryOp::Add,
                    right: Box::new(part),
                },
                span,
            };
        }
        Ok(expression)
    }

    fn binary(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.unary()?;
        loop {
            self.skip_newlines();
            if self.at(&TokenKind::PipeForward) {
                if min_prec > 0 {
                    break;
                }
                self.advance();
                let stage = self.unary()?;
                left = pipeline_call(left, stage)?;
                continue;
            }
            if min_prec == 0 && self.at(&TokenKind::DotDot) {
                self.advance();
                let right = self.binary(1)?;
                let span = left.span.join(right.span);
                left = Expr {
                    kind: ExprKind::Call {
                        name: "range".into(),
                        type_args: Vec::new(),
                        args: vec![left, right],
                    },
                    span,
                };
                continue;
            }
            let (op, prec) = match self.current().kind {
                TokenKind::OrOr => (BinaryOp::Or, 1),
                TokenKind::AndAnd => (BinaryOp::And, 2),
                TokenKind::EqualEqual => (BinaryOp::Equal, 3),
                TokenKind::NotEqual => (BinaryOp::NotEqual, 3),
                TokenKind::Less => (BinaryOp::Less, 4),
                TokenKind::LessEqual => (BinaryOp::LessEqual, 4),
                TokenKind::Greater => (BinaryOp::Greater, 4),
                TokenKind::GreaterEqual => (BinaryOp::GreaterEqual, 4),
                TokenKind::Plus => (BinaryOp::Add, 5),
                TokenKind::Minus => (BinaryOp::Subtract, 5),
                TokenKind::Star => (BinaryOp::Multiply, 6),
                TokenKind::Slash => (BinaryOp::Divide, 6),
                TokenKind::Percent => (BinaryOp::Remainder, 6),
                _ => break,
            };
            if prec < min_prec {
                break;
            }
            self.advance();
            let right = self.binary(prec + 1);
            let right = right?;
            let span = left.span.join(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Expr, ParseError> {
        self.with_nesting(Self::unary_inner)
    }

    fn unary_inner(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();
        if self.at(&TokenKind::Await) {
            let start = self.advance().span;
            let expression = self.unary()?;
            return Ok(Expr {
                kind: ExprKind::Await(Box::new(expression.clone())),
                span: start.join(expression.span),
            });
        }
        if self.at(&TokenKind::Minus) {
            let span = self.advance().span;
            if self.at(&TokenKind::IntMinMagnitude) {
                let magnitude = self.advance().span;
                return Ok(Expr {
                    kind: ExprKind::Int(i64::MIN),
                    span: span.join(magnitude),
                });
            }
            let expr = self.unary()?;
            return Ok(Expr {
                span: span.join(expr.span),
                kind: ExprKind::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                },
            });
        }
        if self.at(&TokenKind::Bang) {
            let span = self.advance().span;
            let expr = self.unary()?;
            return Ok(Expr {
                span: span.join(expr.span),
                kind: ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
            });
        }
        let mut expression = self.primary()?;
        loop {
            if self.at(&TokenKind::LBracket) {
                self.advance();
                let index = self.expression()?;
                let end = self.expect(TokenKind::RBracket, "`]` after array index")?;
                let span = expression.span.join(end);
                expression = Expr {
                    kind: ExprKind::Index {
                        target: Box::new(expression),
                        index: Box::new(index),
                    },
                    span,
                };
            } else if self.at(&TokenKind::Dot) {
                self.advance();
                let (field, end) = self.field_ident("field name after `.`")?;
                let span = expression.span.join(end);
                expression = Expr {
                    kind: ExprKind::Field {
                        target: Box::new(expression),
                        field,
                    },
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expression)
    }
    fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::If => {
                let condition = self.expression()?;
                let then_value = self.expression_block_value("after `if` condition")?;
                self.expect(TokenKind::Else, "`else` after an `if` expression")?;
                let else_value = if self.at(&TokenKind::LBrace) {
                    self.expression_block_value("after `else`")?
                } else if self.at(&TokenKind::If) {
                    self.primary()?
                } else {
                    return self.error("expected an expression block after `else`");
                };
                let span = token.span.join(else_value.span);
                Ok(Expr {
                    kind: ExprKind::Match {
                        value: Box::new(condition),
                        arms: vec![
                            MatchExprArm {
                                pattern: Pattern {
                                    kind: PatternKind::Bool(true),
                                    span: token.span,
                                },
                                guard: None,
                                value: then_value,
                                span: token.span,
                            },
                            MatchExprArm {
                                pattern: Pattern {
                                    kind: PatternKind::Bool(false),
                                    span: token.span,
                                },
                                guard: None,
                                value: else_value,
                                span,
                            },
                        ],
                    },
                    span,
                })
            }
            TokenKind::Pipe => {
                let mut params = Vec::new();
                self.skip_newlines();
                while !self.at(&TokenKind::Pipe) && !self.at(&TokenKind::Eof) {
                    let (name, start) = self.ident("lambda parameter")?;
                    let (ty, end) = if self.at(&TokenKind::Colon) {
                        self.advance();
                        let start = self.current().span;
                        let ty = self.type_name()?;
                        let end = self.tokens[self.pos - 1].span;
                        (ty, start.join(end))
                    } else {
                        (Type::Unknown, start)
                    };
                    params.push(Param {
                        name,
                        ty,
                        span: start.join(end),
                    });
                    self.skip_newlines();
                    if !self.at(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                    self.skip_newlines();
                }
                self.expect(TokenKind::Pipe, "`|` after lambda parameters")?;
                let body = self.expression()?;
                Ok(Expr {
                    span: token.span.join(body.span),
                    kind: ExprKind::Lambda {
                        params,
                        body: Box::new(body),
                    },
                })
            }
            TokenKind::Int(value) => Ok(Expr {
                kind: ExprKind::Int(value),
                span: token.span,
            }),
            TokenKind::IntMinMagnitude => Err(ParseError {
                message: "the Int minimum magnitude is valid only after unary `-`".into(),
                span: token.span,
            }),
            TokenKind::Float(value) => Ok(Expr {
                kind: ExprKind::Float(value),
                span: token.span,
            }),
            TokenKind::String(value) => Ok(Expr {
                kind: ExprKind::String(value),
                span: token.span,
            }),
            TokenKind::StringTemplate { value, raw } => {
                self.interpolated_string(&value, &raw, token.span)
            }
            TokenKind::Char(value) => Ok(Expr {
                kind: ExprKind::Char(value),
                span: token.span,
            }),
            TokenKind::True => Ok(Expr {
                kind: ExprKind::Bool(true),
                span: token.span,
            }),
            TokenKind::False => Ok(Expr {
                kind: ExprKind::Bool(false),
                span: token.span,
            }),
            TokenKind::Match => {
                let value = self.expression()?;
                self.expect(TokenKind::LBrace, "`{` after match value")?;
                let mut arms = Vec::new();
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let pattern = self.pattern()?;
                    let guard = if self.at(&TokenKind::If) {
                        self.advance();
                        Some(self.expression()?)
                    } else {
                        None
                    };
                    self.expect(TokenKind::FatArrow, "`=>` after match pattern")?;
                    let arm_value = self.expression()?;
                    let span = pattern.span.join(arm_value.span);
                    arms.push(MatchExprArm {
                        pattern,
                        guard,
                        value: arm_value,
                        span,
                    });
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                    }
                    self.skip_newlines();
                }
                if arms.is_empty() {
                    return self.error("match expression must contain at least one arm");
                }
                let end = self.expect(TokenKind::RBrace, "`}` after match arms")?;
                Ok(Expr {
                    kind: ExprKind::Match {
                        value: Box::new(value),
                        arms,
                    },
                    span: token.span.join(end),
                })
            }
            TokenKind::LBracket => {
                let mut values = Vec::new();
                self.skip_newlines();
                if !self.at(&TokenKind::RBracket) {
                    loop {
                        values.push(self.expression()?);
                        self.skip_newlines();
                        if !self.at(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                        self.skip_newlines();
                        if self.at(&TokenKind::RBracket) {
                            break;
                        }
                    }
                }
                let end = self.expect(TokenKind::RBracket, "`]` after array literal")?;
                Ok(Expr {
                    kind: ExprKind::Array(values),
                    span: token.span.join(end),
                })
            }
            TokenKind::Sql => self.sql_expression(token.span),
            TokenKind::Ampersand => {
                let (mut name, mut end) = self.ident("function name after `&`")?;
                while self.at(&TokenKind::DoubleColon) {
                    self.advance();
                    let (segment, segment_span) = self.ident("function name after `::`")?;
                    name.push_str("::");
                    name.push_str(&segment);
                    end = segment_span;
                }
                Ok(Expr {
                    kind: ExprKind::FunctionValue(name),
                    span: token.span.join(end),
                })
            }
            TokenKind::Max | TokenKind::Sort | TokenKind::Unique | TokenKind::Filter => {
                let name = if matches!(token.kind, TokenKind::Max) {
                    "max".to_string()
                } else if matches!(token.kind, TokenKind::Filter) {
                    "filter".to_string()
                } else if matches!(token.kind, TokenKind::Sort) {
                    "sort".to_string()
                } else {
                    "unique".to_string()
                };
                if self.at(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    self.skip_newlines();
                    if !self.at(&TokenKind::RParen) {
                        loop {
                            args.push(self.expression()?);
                            self.skip_newlines();
                            if !self.at(&TokenKind::Comma) {
                                break;
                            }
                            self.advance();
                            self.skip_newlines();
                        }
                    }
                    let end = self.expect(TokenKind::RParen, "`)`")?;
                    Ok(Expr {
                        kind: ExprKind::Call {
                            name,
                            type_args: Vec::new(),
                            args,
                        },
                        span: token.span.join(end),
                    })
                } else {
                    Ok(Expr {
                        kind: ExprKind::Variable(name),
                        span: token.span,
                    })
                }
            }
            TokenKind::Ident(name) => {
                let mut name = name;
                while self.at(&TokenKind::DoubleColon) {
                    self.advance();
                    let (segment, _) = self.ident("name after `::`")?;
                    name.push_str("::");
                    name.push_str(&segment);
                }
                let record_type_args =
                    if self.at(&TokenKind::Less) && self.looks_like_generic_record_literal() {
                        self.advance();
                        let mut type_args = Vec::new();
                        loop {
                            type_args.push(self.type_name()?);
                            if !self.at(&TokenKind::Comma) {
                                break;
                            }
                            self.advance();
                        }
                        self.expect(TokenKind::Greater, "`>` after record type arguments")?;
                        type_args
                    } else {
                        Vec::new()
                    };
                let type_args = if self.at(&TokenKind::Less)
                    && (matches!(
                        name.as_str(),
                        "json_decode"
                            | "json_decode_result"
                            | "sql_decode"
                            | "json_decode_tagged_result"
                            | "http_json"
                            | "http_result"
                            | "module_setting"
                    ) || self.looks_like_generic_call())
                {
                    self.advance();
                    let mut type_args = Vec::new();
                    loop {
                        type_args.push(self.type_name()?);
                        if !self.at(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                    self.expect(TokenKind::Greater, "`>` after call type arguments")?;
                    type_args
                } else {
                    Vec::new()
                };
                if self.at(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    self.skip_newlines();
                    if !self.at(&TokenKind::RParen) {
                        loop {
                            args.push(self.expression()?);
                            self.skip_newlines();
                            if !self.at(&TokenKind::Comma) {
                                break;
                            }
                            self.advance();
                            self.skip_newlines();
                        }
                    }
                    let end = self.expect(TokenKind::RParen, "`)`")?;
                    Ok(Expr {
                        kind: ExprKind::Call {
                            name,
                            type_args,
                            args,
                        },
                        span: token.span.join(end),
                    })
                } else if !type_args.is_empty() {
                    self.error("type arguments require a function call")
                } else if name == "Map" && self.at(&TokenKind::LBrace) {
                    self.map_literal(token.span)
                } else if self.at(&TokenKind::LBrace) && self.looks_like_record_literal() {
                    self.record_literal(name, record_type_args, token.span)
                } else {
                    if !record_type_args.is_empty() {
                        return self.error("generic type arguments require a record literal here");
                    }
                    Ok(Expr {
                        kind: ExprKind::Variable(name),
                        span: token.span,
                    })
                }
            }
            TokenKind::LParen => {
                let first = self.expression()?;
                if !self.at(&TokenKind::Comma) {
                    self.expect(TokenKind::RParen, "`)`")?;
                    return Ok(first);
                }
                let mut args = vec![first];
                while self.at(&TokenKind::Comma) {
                    self.advance();
                    self.skip_newlines();
                    if self.at(&TokenKind::RParen) {
                        break;
                    }
                    args.push(self.expression()?);
                    self.skip_newlines();
                }
                let end = self.expect(TokenKind::RParen, "`)` after tuple values")?;
                if args.len() < 2 {
                    return Err(ParseError {
                        message: "a tuple requires at least two values".into(),
                        span: token.span.join(end),
                    });
                }
                Ok(Expr {
                    kind: ExprKind::Call {
                        name: "tuple".into(),
                        type_args: Vec::new(),
                        args,
                    },
                    span: token.span.join(end),
                })
            }
            found => self.error(format!("expected expression, found {found:?}")),
        }
    }

    /// Parses the deliberately expression-only body of an `if` expression.
    /// Statement `if` blocks retain their existing multi-statement grammar.
    fn expression_block_value(&mut self, context: &str) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace, &format!("`{{` {context}"))?;
        self.skip_newlines();
        let value = self.expression()?;
        self.skip_newlines();
        if self.at(&TokenKind::Semicolon) {
            self.advance();
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after if-expression value")?;
        Ok(value)
    }

    fn record_literal(
        &mut self,
        type_name: String,
        type_args: Vec<Type>,
        start: Span,
    ) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after record type")?;
        let mut fields = Vec::new();
        let mut update_field_spans = Vec::new();
        let mut base = None;
        self.skip_newlines();
        if self.at(&TokenKind::DotDot) {
            self.advance();
            base = Some(self.expression()?);
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::DotDot) {
                return self.error("record update spread must appear once, before explicit fields");
            }
            let (field, field_span) = self.field_ident("record field name")?;
            self.expect(TokenKind::Colon, "`:` after record field name")?;
            let value = self.expression()?;
            if fields.iter().any(|(existing, _)| existing == &field) {
                return Err(ParseError {
                    message: format!("record field `{field}` is specified more than once"),
                    span: field_span,
                });
            }
            if base.is_some() {
                update_field_spans.push(field_span);
            }
            fields.push((field, value));
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after record literal")?;
        if let Some(base) = base {
            let expected_type = if type_args.is_empty() {
                Type::Named(type_name)
            } else {
                Type::Applied {
                    name: type_name,
                    args: type_args,
                }
            };
            let mut args = vec![base];
            for ((field, value), field_span) in fields.into_iter().zip(update_field_spans) {
                args.push(Expr {
                    kind: ExprKind::String(field),
                    span: field_span,
                });
                args.push(value);
            }
            return Ok(Expr {
                kind: ExprKind::Call {
                    name: "record_update".into(),
                    type_args: vec![expected_type],
                    args,
                },
                span: start.join(end),
            });
        }
        Ok(Expr {
            kind: ExprKind::Record {
                type_name,
                type_args,
                fields,
            },
            span: start.join(end),
        })
    }

    fn looks_like_generic_record_literal(&self) -> bool {
        if !self.at(&TokenKind::Less) {
            return false;
        }
        let mut depth = 0_usize;
        for (offset, token) in self.tokens[self.pos..].iter().enumerate() {
            match token.kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self
                            .tokens
                            .get(self.pos + offset + 1)
                            .is_some_and(|next| next.kind == TokenKind::LBrace);
                    }
                }
                TokenKind::Eof | TokenKind::Newline => return false,
                _ => {}
            }
        }
        false
    }

    fn map_literal(&mut self, start: Span) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after `Map`")?;
        let mut entries = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let key = self.expression()?;
            self.expect(TokenKind::Colon, "`:` after Map key")?;
            let value = self.expression()?;
            entries.push((key, value));
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after Map literal")?;
        Ok(Expr {
            kind: ExprKind::Map(entries),
            span: start.join(end),
        })
    }

    fn looks_like_record_literal(&self) -> bool {
        let mut position = self.pos + 1;
        while matches!(
            self.tokens.get(position).map(|token| &token.kind),
            Some(TokenKind::Newline | TokenKind::Semicolon)
        ) {
            position += 1;
        }
        matches!(
            (
                self.tokens.get(position).map(|token| &token.kind),
                self.tokens.get(position + 1).map(|token| &token.kind)
            ),
            (
                Some(TokenKind::Ident(_) | TokenKind::Type | TokenKind::Version),
                Some(TokenKind::Colon)
            ) | (Some(TokenKind::DotDot), _)
        )
    }

    fn sql_expression(&mut self, start: Span) -> Result<Expr, ParseError> {
        let result_type = if self.at(&TokenKind::Less) {
            self.advance();
            let result_type = self.type_name()?;
            self.expect(TokenKind::Greater, "`>` after SQL result type")?;
            result_type
        } else {
            Type::Unit
        };
        self.expect(TokenKind::LBrace, "`{` after SQL block header")?;
        let query = match self.current().kind.clone() {
            TokenKind::SqlBody(query) => {
                self.advance();
                query
            }
            _ => return self.error("expected SQL query body"),
        };
        let end = self.expect(TokenKind::RBrace, "`}` after SQL query")?;
        Ok(Expr {
            kind: ExprKind::Sql { result_type, query },
            span: start.join(end),
        })
    }
}

fn interpolation_end(value: &str, start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut quote = None;
    let mut escaped = false;
    let mut cursor = start;
    while cursor < value.len() {
        let character = value[cursor..].chars().next()?;
        let width = character.len_utf8();
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '"' || character == '\'' {
            quote = Some(character);
        } else if character == '{' {
            depth += 1;
        } else if character == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += width;
    }
    None
}

fn decoded_string_source_offsets(
    raw: &str,
    decoded: &str,
    source_start: usize,
) -> Option<Vec<usize>> {
    let mut offsets = vec![0; decoded.len() + 1];
    offsets[0] = source_start;
    let mut raw_cursor = 0;
    let mut decoded_cursor = 0;

    while raw_cursor < raw.len() {
        let raw_start = raw_cursor;
        let raw_character = raw[raw_cursor..].chars().next()?;
        let (decoded_character, raw_end) = if raw_character == '\\' {
            raw_cursor += 1;
            let escaped = raw[raw_cursor..].chars().next()?;
            let decoded_character = match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                '\'' => '\'',
                _ => return None,
            };
            raw_cursor += escaped.len_utf8();
            (decoded_character, raw_cursor)
        } else {
            raw_cursor += raw_character.len_utf8();
            (raw_character, raw_cursor)
        };

        if !decoded[decoded_cursor..].starts_with(decoded_character) {
            return None;
        }
        let decoded_end = decoded_cursor + decoded_character.len_utf8();
        let raw_start_absolute = source_start + raw_start;
        let raw_end_absolute = source_start + raw_end;
        for byte in 1..=decoded_character.len_utf8() {
            offsets[decoded_cursor + byte] = if byte == decoded_character.len_utf8() {
                raw_end_absolute
            } else {
                raw_start_absolute + byte
            };
        }
        decoded_cursor = decoded_end;
    }

    (decoded_cursor == decoded.len()).then_some(offsets)
}

fn interpolation_source_span(start: usize, end: usize, raw: &str, string_span: Span) -> Span {
    let start_raw = start.saturating_sub(string_span.start + 1).min(raw.len());
    let position = |raw_offset: usize| {
        let prefix = &raw[..raw_offset];
        let line_offset = prefix.bytes().filter(|byte| *byte == b'\n').count();
        let line = string_span.line + line_offset;
        let column = prefix
            .rfind('\n')
            .map_or(string_span.column + 1 + prefix.chars().count(), |newline| {
                prefix[newline + 1..].chars().count() + 1
            });
        (line, column)
    };
    let (line, column) = position(start_raw);
    Span {
        start,
        end,
        line,
        column,
        source_id: string_span.source_id,
    }
}

fn push_interpolation_text(parts: &mut Vec<Expr>, text: &str, span: Span) {
    if text.is_empty() {
        return;
    }
    parts.push(Expr {
        kind: ExprKind::String(text.replace("{{", "{").replace("}}", "}")),
        span,
    });
}

fn template_literal(value: &str, span: Span) -> Expr {
    Expr {
        kind: ExprKind::String(value.replace("{{", "{").replace("}}", "}")),
        span,
    }
}

fn pipeline_call(input: Expr, stage: Expr) -> Result<Expr, ParseError> {
    let (name, type_args, mut args) = match stage.kind {
        ExprKind::Variable(name) | ExprKind::FunctionValue(name) => (
            pipeline_builtin_alias(&name).to_owned(),
            Vec::new(),
            Vec::new(),
        ),
        ExprKind::Call {
            name,
            type_args,
            args,
        } => (pipeline_builtin_alias(&name).to_owned(), type_args, args),
        _ => {
            return Err(ParseError {
                message: "pipeline stage must name a function or a function call".into(),
                span: stage.span,
            })
        }
    };
    args.insert(0, input.clone());
    Ok(Expr {
        kind: ExprKind::Call {
            name,
            type_args,
            args,
        },
        span: input.span.join(stage.span),
    })
}

fn pipeline_builtin_alias(name: &str) -> &str {
    match name {
        "trim" => "text_trim",
        "to_lower" => "text_to_lower",
        "to_upper" => "text_to_upper",
        _ => name,
    }
}

fn parse_file_size(input: &str) -> Option<u64> {
    let input = input.trim();
    let split = input
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(input.len());
    let (number, suffix) = input.split_at(split);
    let number = number.parse::<u64>().ok()?;
    if number == 0 {
        return None;
    }
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "b" => 1_u64,
        "kb" => 1_000,
        "mb" => 1_000_000,
        "kib" => 1_024,
        "mib" => 1_048_576,
        _ => return None,
    };
    number.checked_mul(multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    #[test]
    fn parses_function_and_expression_precedence() {
        let program = parse(&lex("fn main() { print(1 + 2 * 3) }").unwrap()).unwrap();
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn suggests_block_syntax_for_else_if_without_a_block() {
        let source = "fn main() { if ready { print(1) } else if retry print(2) }";
        let error = parse(&lex(source).unwrap()).unwrap_err();

        assert_eq!(
            error.message,
            "expected a block after `else if`; use `else if condition { ... }`"
        );
        assert_eq!(error.span.start, source.find("print(2)").unwrap());
        assert_eq!(error.span.end, error.span.start + "print".len());
    }

    #[test]
    fn parses_pipeline_stages_as_left_to_right_function_calls() {
        let program =
            parse(&lex("fn main() { result = \"  HI \" |> trim |> to_lower }").unwrap()).unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected pipeline binding");
        };
        let ExprKind::Call {
            name: outer_name,
            args: outer_args,
            ..
        } = &value.kind
        else {
            panic!("expected outer pipeline call");
        };
        assert_eq!(outer_name, "text_to_lower");
        let ExprKind::Call {
            name: inner_name,
            args: inner_args,
            ..
        } = &outer_args[0].kind
        else {
            panic!("expected inner pipeline call");
        };
        assert_eq!(inner_name, "text_trim");
        assert!(matches!(inner_args[0].kind, ExprKind::String(ref text) if text == "  HI "));
    }

    #[test]
    fn parses_if_expressions_as_exhaustive_boolean_matches() {
        let program = parse(
            &lex("fn main() { result = if ready { 1 } else if retry { 2 } else { 3 } }").unwrap(),
        )
        .unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected a binding");
        };
        let ExprKind::Match { value, arms } = &value.kind else {
            panic!("if expression should lower to a boolean match");
        };
        assert!(matches!(value.kind, ExprKind::Variable(ref name) if name == "ready"));
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].pattern.kind, PatternKind::Bool(true)));
        assert!(matches!(arms[1].pattern.kind, PatternKind::Bool(false)));
        assert!(matches!(arms[1].value.kind, ExprKind::Match { .. }));
    }

    #[test]
    fn parses_contextual_record_fields_and_record_update_spreads() {
        let program = parse(
            &lex(
                "struct Invoice { version: Int type: String status: Bool } fn main() { original = Invoice { version: 7, type: \"sale\", status: true }; changed = Invoice { ..original, status: false }; print(changed.version) }",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.records[0]
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["version", "type", "status"]
        );
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[1] else {
            panic!("expected the record update binding");
        };
        let ExprKind::Call {
            name,
            type_args,
            args,
        } = &value.kind
        else {
            panic!("record update syntax should lower to the checked update intrinsic");
        };
        assert_eq!(name, "record_update");
        assert_eq!(type_args, &[Type::Named("Invoice".into())]);
        assert_eq!(args.len(), 3);
        assert!(matches!(args[1].kind, ExprKind::String(ref field) if field == "status"));
        assert_ne!(
            args[1].span, args[2].span,
            "the synthetic field-name expression needs its own source span"
        );
    }

    #[test]
    fn parses_tuple_literals_while_preserving_parenthesized_expressions() {
        let program = parse(
            &lex("fn coordinates() -> Tuple<Int, Int> { return (4, 9) } fn main() { point = (1 + 2) }")
                .unwrap(),
        )
        .unwrap();
        let Stmt::Return {
            value: Some(tuple), ..
        } = &program.functions[0].body.statements[0]
        else {
            panic!("expected tuple return");
        };
        assert!(
            matches!(tuple.kind, ExprKind::Call { ref name, ref args, .. } if name == "tuple" && args.len() == 2)
        );
        let Stmt::BindOrAssign { value, .. } = &program.functions[1].body.statements[0] else {
            panic!("expected grouped arithmetic value");
        };
        assert!(matches!(value.kind, ExprKind::Binary { .. }));
    }

    #[test]
    fn rejects_non_call_pipeline_stages_at_the_stage_span() {
        let source = "fn main() { value = \"text\" |> (1 + 2) }";
        let error = parse(&lex(source).unwrap()).unwrap_err();

        assert_eq!(
            error.message,
            "pipeline stage must name a function or a function call"
        );
        let stage_start = source.find("1 + 2").unwrap();
        assert_eq!(error.span.start, stage_start);
        assert_eq!(error.span.end, stage_start + "1 + 2".len());
    }

    #[test]
    fn parses_generic_function_declarations_and_calls() {
        let program = parse(
            &lex("fn identity<T>(value: T) -> T { return value } fn main() { print(identity<Int>(7)) }")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(program.functions[0].type_params, ["T"]);
        let Stmt::Expr(outer) = &program.functions[1].body.statements[0] else {
            panic!("expected call expression");
        };
        let ExprKind::Call { name, args, .. } = &outer.kind else {
            panic!("expected outer print call");
        };
        assert_eq!(name, "print");
        let ExprKind::Call {
            name, type_args, ..
        } = &args[0].kind
        else {
            panic!("expected inner generic call");
        };
        assert_eq!(name, "identity");
        assert_eq!(type_args, &[Type::Int]);
    }

    #[test]
    fn parses_named_function_values_and_typed_function_parameters() {
        let program = parse(
            &lex("fn increment(value: Int) -> Int { return value + 1 } fn apply(callback: Function<(Int) -> Int>, value: Int) -> Int { return callback(value) } fn main() { saved = &increment print(apply(saved, 4)) }")
                .unwrap(),
        )
        .unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[2].body.statements[0] else {
            panic!("expected local function value");
        };
        assert!(matches!(value.kind, ExprKind::FunctionValue(ref name) if name == "increment"));
        assert!(matches!(
            program.functions[1].params[0].ty,
            Type::Function { ref params, ref return_type, ref capabilities }
                if params == &[Type::Int] && **return_type == Type::Int && capabilities.is_empty()
        ));

        let effectful = parse(
            &lex("fn run_file(path: String) -> String uses FileSystem { return path } fn use_file(callback: Function<(String) -> String> uses {FileSystem}) { print(callback(\"x\")) } fn main() { }")
                .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            effectful.functions[1].params[0].ty,
            Type::Function { ref capabilities, .. } if capabilities == &["FileSystem"]
        ));

        let compatibility = parse(
            &lex("fn increment(value: Int) -> Int { return value + 1 } fn function() -> Int { return 9 } fn main() { function = &increment print(function(2)) }")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(compatibility.functions[1].name, "function");
        assert!(matches!(
            compatibility.functions[2].body.statements[0],
            Stmt::BindOrAssign { ref name, .. } if name == "function"
        ));
    }

    #[test]
    fn parses_payload_enum_definitions_constructors_and_patterns() {
        let program = parse(
            &lex("enum OrderStatus { Draft PaidAt(Date) Failed(String, Int) } fn main() { status = OrderStatus::PaidAt(date(\"2026-10-12\")) match status { OrderStatus::PaidAt(when) => { print(when) } OrderStatus::Failed(reason, code) => { print(reason) } OrderStatus::Draft => { print(\"draft\") } } }")
                .unwrap(),
        )
        .unwrap();
        let definition = &program.enums[0];
        assert_eq!(definition.variants, ["Draft", "PaidAt", "Failed"]);
        assert_eq!(
            definition.variant_payloads,
            [
                ("Draft".into(), vec![]),
                ("PaidAt".into(), vec![Type::Date]),
                ("Failed".into(), vec![Type::String, Type::Int]),
            ]
        );
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected enum construction assignment");
        };
        assert!(
            matches!(value.kind, ExprKind::Call { ref name, .. } if name == "OrderStatus::PaidAt")
        );
        let Stmt::Match { arms, .. } = &program.functions[0].body.statements[1] else {
            panic!("expected enum match");
        };
        assert!(matches!(
            arms[0].pattern.kind,
            PatternKind::EnumVariantPayload { ref enum_name, ref variant, ref payloads }
                if enum_name == "OrderStatus" && variant == "PaidAt" && payloads.len() == 1
        ));

        let imported = parse(
            &lex("fn main() { match event { events::OrderStatus::PaidAt(when) => { print(when) } } }")
                .unwrap(),
        )
        .unwrap();
        let Stmt::Match { arms, .. } = &imported.functions[0].body.statements[0] else {
            panic!("expected match statement");
        };
        assert!(matches!(
            arms[0].pattern.kind,
            PatternKind::EnumVariantPayload { ref enum_name, ref variant, .. }
                if enum_name == "events::OrderStatus" && variant == "PaidAt"
        ));

        let empty = parse(&lex("enum Status { Draft() } fn main() { }").unwrap())
            .expect_err("unit enum variants use the bare variant name");
        assert!(empty.message.contains("empty parentheses"));
        assert!(empty.span.start > 0);
    }

    #[test]
    fn rejects_duplicate_generic_function_parameters() {
        let error = parse(
            &lex("fn pair<T, T>(left: T, right: T) -> T { return left } fn main() { }").unwrap(),
        )
        .expect_err("generic parameter names must be unique");
        assert!(error.message.contains("declared more than once"));
        assert!(error.span.start > 0);
    }

    #[test]
    fn parses_min_int_literal_with_whitespace_and_parenthesized_expression() {
        for source in [
            "fn main() { value = - 9223372036854775808 }",
            "fn main() { value = (-9223372036854775808) }",
        ] {
            let program = parse(&lex(source).unwrap()).unwrap();
            let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
                panic!("expected binding expression");
            };
            assert!(matches!(value.kind, ExprKind::Int(i64::MIN)));
        }
        let error = parse(&lex("fn main() { value = -(9223372036854775808) }").unwrap())
            .expect_err("an out-of-range positive inner literal must not be accepted");
        assert!(error.message.contains("valid only after unary `-`"));
        assert!(error.span.start > 0);
    }
    #[test]
    fn parses_nested_array_types_for_tabular_text_data() {
        let program = parse(
            &lex("fn rows() -> String[][] { return [[\"one\", \"two\"], [\"three\", \"four\"]] }")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.functions[0].return_type,
            Some(Type::Array(Box::new(Type::Array(Box::new(Type::String)))))
        );
    }
    #[test]
    fn incomplete_expression_at_end_of_input_returns_a_parse_error() {
        let tokens = lex("fn e(){n*").unwrap();
        let error = parse(&tokens).unwrap_err();
        assert!(error.message.contains("expected expression"));
    }

    #[test]
    fn fuzzed_recursive_input_returns_a_parse_error() {
        let source = std::str::from_utf8(include_bytes!(
            "../../fuzz/corpus/lexer_parser/parser_recursive_stack_overflow"
        ))
        .unwrap();
        let tokens = lex(source).unwrap();
        let error = parse(&tokens).unwrap_err();
        assert!(error.message.contains("maximum parser nesting depth"));
    }

    #[test]
    fn excessive_recursive_syntax_returns_a_parse_error() {
        let depth = 256;
        let cases = [
            format!(
                "fn main() {{ value = {}0{} }}",
                "[".repeat(depth),
                "]".repeat(depth)
            ),
            format!("fn main() {{ value = {}true }}", "!".repeat(depth)),
            format!(
                "fn main() {{ {} }}",
                "if true { ".repeat(depth) + &"}".repeat(depth)
            ),
            format!(
                "fn main(value: {}Int{}) {{}}",
                "Option<".repeat(depth),
                ">".repeat(depth)
            ),
            format!(
                "fn main() {{ match value {{ {}value{} => {{}} }} }}",
                "Some(".repeat(depth),
                ")".repeat(depth)
            ),
        ];

        for source in cases {
            let tokens = lex(&source).unwrap();
            let error = parse(&tokens).unwrap_err();
            assert!(
                error.message.contains("maximum parser nesting depth"),
                "unexpected error for deeply nested source: {}",
                error.message
            );
        }
    }

    #[test]
    fn parse_adds_an_eof_token_when_callers_omit_it() {
        let tokens = lex("fn main() {}").unwrap();
        let without_eof = &tokens[..tokens.len() - 1];
        let program = parse(without_eof).unwrap();
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn parses_multiline_return() {
        let source = "fn f(n: Int) -> Int { return n +\n 1 }";
        assert!(parse(&lex(source).unwrap()).is_ok());
    }

    #[test]
    fn parses_loop_control_statements() {
        let program = parse(&lex("fn main() { while true { continue break } }").unwrap()).unwrap();
        let body = match &program.functions[0].body.statements[0] {
            Stmt::While { body, .. } => body,
            statement => panic!("expected while, found {statement:?}"),
        };
        assert!(matches!(body.statements[0], Stmt::Continue { .. }));
        assert!(matches!(body.statements[1], Stmt::Break { .. }));
    }

    #[test]
    fn parses_loop_invariants() {
        let program = parse(
            &lex("fn main() { mutable i = 0 while i < 3 invariant { i >= 0 } { i = i + 1 } }")
                .unwrap(),
        )
        .unwrap();
        let Stmt::While { invariants, .. } = &program.functions[0].body.statements[1] else {
            panic!("expected while statement");
        };
        assert_eq!(invariants.len(), 1);
    }

    #[test]
    fn parses_invariants_on_unconditional_loops() {
        let program =
            parse(&lex("fn main() { loop invariant { true } { break } }").unwrap()).unwrap();
        let Stmt::Loop { invariants, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected loop statement");
        };
        assert_eq!(invariants.len(), 1);
    }

    #[test]
    fn parses_multiple_loop_invariants() {
        let program = parse(
            &lex("fn main() { loop invariant { true } invariant { 1 < 2 } { break } }").unwrap(),
        )
        .unwrap();
        let Stmt::Loop { invariants, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected loop statement");
        };
        assert_eq!(invariants.len(), 2);
    }

    #[test]
    fn parses_native_sql_and_transactions() {
        let source = "fn main() { transaction { sql { UPDATE customers SET name = :name WHERE id = :id } } rows = sql<Customer[]> { SELECT id, name FROM customers } }";
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(matches!(
            program.functions[0].body.statements[0],
            Stmt::Transaction { .. }
        ));
        assert!(matches!(
            program.functions[0].body.statements[1],
            Stmt::BindOrAssign { .. }
        ));
    }

    #[test]
    fn parses_page_with_html_template() {
        let source = r#"
            page "/hello/{name}" {
                html {
                    <h1>Hello, {name}!</h1>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages.len(), 1);
        assert_eq!(program.pages[0].path, "/hello/{name}");
        assert!(program.pages[0].html.contains("{name}"));
    }

    #[test]
    fn parses_typed_page_data_loading() {
        let source = r#"
            page "/customers/{name}" {
                load customer = sql<Customer> {
                    SELECT id, name FROM customers WHERE name = :name
                }
                html {
                    <h1>{customer.name}</h1>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages[0].data.len(), 1);
        assert_eq!(program.pages[0].data[0].name, "customer");
        assert_eq!(
            program.pages[0].data[0].result_type,
            Type::Named("Customer".into())
        );
        assert!(program.pages[0].data[0]
            .query
            .contains("WHERE name = :name"));
    }

    #[test]
    fn parses_typed_page_collection_and_view_loop() {
        let source = r#"
            page "/customers" {
                load customers = sql<Customer[]> {
                    SELECT id, name FROM customers
                }
                html {
                    <ul>
                        for customer in customers {
                            <li>{customer.name}</li>
                        }
                    </ul>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(
            program.pages[0].data[0].result_type,
            Type::Array(Box::new(Type::Named("Customer".into())))
        );
        assert!(program.pages[0].html.contains("for customer in customers"));
    }

    #[test]
    fn parses_typed_page_query_inputs() {
        let source = r#"
            page "/customers" {
                input {
                    search: String?
                    page: UInt
                }
                html { <p>{search}</p> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages[0].inputs.len(), 2);
        assert_eq!(program.pages[0].inputs[0].name, "search");
        assert_eq!(
            program.pages[0].inputs[0].ty,
            Type::Option(Box::new(Type::String))
        );
        assert_eq!(program.pages[0].inputs[1].ty, Type::UInt);
    }

    #[test]
    fn parses_page_pagination() {
        let source = r#"
            page "/customers" {
                paginated 25
                html { <p>Customers</p> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages[0].page_size, Some(25));
    }

    #[test]
    fn parses_page_sort_fields() {
        let source = r#"
            page "/customers" {
                sort { name, created_at }
                html { <p>Customers</p> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages[0].sort, ["name", "created_at"]);
    }

    #[test]
    fn parses_page_search_fields() {
        let source = r#"
            page "/customers" {
                search { name, email }
                html { <p>Customers</p> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages[0].search, ["name", "email"]);
    }

    #[test]
    fn parses_page_filter_fields() {
        let source = r#"
            page "/customers" {
                filter { active, quantity }
                html { <p>Customers</p> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.pages[0].filters, ["active", "quantity"]);
    }

    #[test]
    fn parses_named_view_and_page_view_assignment() {
        let source = r#"
            view AppShell {
                html {
                    <html><body><main><slot /></main></body></html>
                }
            }
            page "/customers" {
                view: AppShell
                html {
                    <h1>Customers</h1>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.views.len(), 1);
        assert_eq!(program.views[0].name, "AppShell");
        assert!(program.views[0].html.contains("<slot />"));
        assert_eq!(program.pages[0].view.as_deref(), Some("AppShell"));
    }

    #[test]
    fn parses_typed_view_component_properties() {
        let source = r#"
            component Badge {
                props {
                    text: String
                    count: Int?
                }
                html { <span>{text}</span> }
            }
            page "/status" {
                html { <Badge text="Ready" /> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.components.len(), 1);
        assert_eq!(program.components[0].props.len(), 2);
        assert_eq!(
            program.components[0].props[1].ty,
            Type::Option(Box::new(Type::Int))
        );
    }

    #[test]
    fn parses_tableview_source_and_controls() {
        let source = r#"
            table customers { id: Id primary auto name: String(100) }
            tableview Customers {
                layout: BusinessShell
                source sql<Customer[]> {
                    SELECT id, name FROM customers
                }
                columns { id name }
                filter { name }
                searchable
                sortable
                paginated 25
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.tableviews.len(), 1);
        let tableview = &program.tableviews[0];
        assert_eq!(tableview.name, "Customers");
        assert_eq!(tableview.layout.as_deref(), Some("BusinessShell"));
        assert_eq!(
            tableview.result_type,
            Type::Array(Box::new(Type::Named("Customer".into())))
        );
        assert_eq!(tableview.columns, ["id", "name"]);
        assert_eq!(tableview.filters, ["name"]);
        assert!(tableview.searchable);
        assert!(tableview.sortable);
        assert_eq!(tableview.page_size, Some(25));
    }

    #[test]
    fn parses_foreign_key_delete_actions_on_table_columns() {
        let source = r#"
            table customers { id: Id primary auto }
            table invoices {
                customer: Customer required on_delete cascade
                template: Template on_delete set_null
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(
            program.tables[1].columns[0].on_delete,
            Some(ReferentialAction::Cascade)
        );
        assert_eq!(
            program.tables[1].columns[1].on_delete,
            Some(ReferentialAction::SetNull)
        );
    }

    #[test]
    fn parses_explicit_scalar_column_references() {
        let source = r#"
            type TenantId = Id
            table tenants { id: TenantId primary auto }
            table users {
                tenant_id: TenantId required references tenants on_delete cascade
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let column = &program.tables[1].columns[0];
        assert_eq!(column.references.as_deref(), Some("tenants"));
        assert_eq!(column.on_delete, Some(ReferentialAction::Cascade));

        let duplicate = r#"table targets { id: Id primary auto } table sources { target_id: Id references targets references targets }"#;
        let error = parse(&lex(duplicate).unwrap()).unwrap_err();
        assert!(error
            .message
            .contains("`references` may be specified only once"));
    }

    #[test]
    fn parses_unique_within_tenant_and_rejects_duplicate_modifiers() {
        let source = "table customers scoped by tenant_id: TenantId { tenant_id: TenantId required customer_number: String unique within tenant }";
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(program.tables[0].columns[1].unique_within_tenant);
        assert!(!program.tables[0].columns[1].unique);

        let duplicate = "table customers scoped by tenant_id: TenantId { tenant_id: TenantId required customer_number: String unique within tenant unique within tenant }";
        let error = parse(&lex(duplicate).unwrap()).unwrap_err();
        assert!(error.message.contains("may be specified only once"));
    }

    #[test]
    fn parses_table_validation_rules_with_static_messages() {
        let source = r#"
            table invoices {
                invoice_date: Date required
                due_date: Date?
                validate {
                    match due_date {
                        Some(due) => due >= invoice_date
                        None => true
                    }: "The due date must not precede the invoice date."
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let validations = &program.tables[0].validations;
        assert_eq!(validations.len(), 1);
        assert_eq!(
            validations[0].message,
            "The due date must not precede the invoice date."
        );
        assert!(validations[0].span.start < validations[0].span.end);

        let interpolated = source.replace(
            "\"The due date must not precede the invoice date.\"",
            "\"Due date {invoice_date}\"",
        );
        assert!(parse(&lex(&interpolated).unwrap()).is_err());
    }

    #[test]
    fn parses_tenant_scoped_table_column_and_identity_type() {
        let program = parse(
            &lex("table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required }\n")
                .unwrap(),
        )
        .unwrap();
        let scope = program.tables[0].tenant_scope.as_ref().unwrap();
        assert_eq!(scope.column, "tenant_id");
        assert_eq!(scope.tenant_type, Type::Named("TenantId".into()));
        assert_eq!(scope.span.line, 1);
    }

    #[test]
    fn accepts_scoped_clause_after_table_name_before_body() {
        let source = "table invoices scoped by tenant_id: TenantId { id: Id primary auto tenant_id: TenantId required }";
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.tables[0].name, "invoices");
        assert!(program.tables[0].tenant_scope.is_some());
    }

    #[test]
    fn rejects_malformed_tenant_scope_clause() {
        let error = parse(
            &lex("table invoices scoped tenant_id: TenantId { id: Id primary auto }\n").unwrap(),
        )
        .unwrap_err();
        assert!(error.message.contains("expected `by` after `scoped`"));
    }

    #[test]
    fn parses_explicit_storage_names_for_relation_columns() {
        let program = parse(&lex(
            "table users { id: Id primary auto } table vacation_requests { approved_by: User storage_name \"approved_by\" on_delete set_null }",
        )
        .unwrap())
        .unwrap();
        let relation = &program.tables[1].columns[0];
        assert_eq!(relation.name, "approved_by");
        assert_eq!(relation.storage_name.as_deref(), Some("approved_by"));
        assert_eq!(relation.on_delete, Some(ReferentialAction::SetNull));
    }

    #[test]
    fn accepts_contextual_database_field_names_and_existing_storage_mapping() {
        let program = parse(
            &lex("table invoices { type: String(20) version: Int order: Int group: Int status: String invoice_type: String(20) storage_name \"invoice_category\" }")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.tables[0]
                .columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            [
                "type",
                "version",
                "order",
                "group",
                "status",
                "invoice_type"
            ]
        );
        assert_eq!(
            program.tables[0].columns[5].storage_name.as_deref(),
            Some("invoice_category")
        );
    }

    #[test]
    fn parses_decimal_precision_and_scale_for_table_columns() {
        let program = parse(
            &lex("table invoices { id: Id primary auto total: Decimal(12, 2) required tax: Decimal(8, 4)? }\n").unwrap(),
        )
        .unwrap();
        assert_eq!(program.tables[0].columns[1].ty, Type::Decimal);
        assert_eq!(
            program.tables[0].columns[1].decimal_precision,
            Some((12, 2))
        );
        assert_eq!(program.tables[0].columns[2].decimal_precision, Some((8, 4)));
        assert_eq!(
            program.tables[0].columns[2].ty,
            Type::Option(Box::new(Type::Decimal))
        );
    }

    #[test]
    fn parses_declarative_business_number_series() {
        let tokens = lex(
            "table customers { customer_number: String(32) required auto_sequence(\"KD-{0001}\") }",
        )
        .unwrap();
        let program = parse(&tokens).expect("number-series column should parse");
        assert_eq!(
            program.tables[0].columns[0].auto_sequence.as_deref(),
            Some("KD-{0001}")
        );
        let tokens = lex(
            "table quotes { quote_number: String required auto_sequence(\"AG-{year}-{0001}\") }",
        )
        .unwrap();
        let program = parse(&tokens).expect("year token should parse");
        assert_eq!(
            program.tables[0].columns[0].auto_sequence.as_deref(),
            Some("AG-{year}-{0001}")
        );
    }

    #[test]
    fn parses_computed_column_expressions() {
        let program = parse(
            &lex("table invoice_items { quantity: Decimal(10, 2) required unit_price: Decimal(12, 2) required total_net: Decimal(12, 2) required compute(quantity * unit_price) }\n").unwrap(),
        )
        .expect("computed column expression should parse");
        let total = &program.tables[0].columns[2];
        assert!(total.computed.is_some());
        assert!(
            total.computed.as_ref().unwrap().span.end > total.computed.as_ref().unwrap().span.start
        );
    }

    #[test]
    fn parses_json_columns_as_the_native_json_type() {
        let program = parse(
            &lex("table events { id: Id primary payload: Json required metadata: Json? }\n")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(program.tables[0].columns[1].ty, Type::Json);
        assert_eq!(
            program.tables[0].columns[2].ty,
            Type::Option(Box::new(Type::Json))
        );
    }

    #[test]
    fn parses_contextual_type_and_version_table_column_names() {
        let program = parse(
            &lex("table metadata { id: Id primary auto type: String(32) version: Int required }\n")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(program.tables[0].columns[1].name, "type");
        assert_eq!(program.tables[0].columns[2].name, "version");
    }

    #[test]
    fn parses_sessions_as_a_table_name_outside_auth_configuration() {
        let program = parse(&lex("table sessions { id: String primary }").unwrap()).unwrap();
        assert_eq!(program.tables[0].name, "sessions");
        assert_eq!(program.tables[0].columns[0].name, "id");
        assert!(program.tables[0].columns[0].primary_key);
        assert!(!program.tables[0].columns[0].auto);
    }

    #[test]
    fn parses_contextual_type_and_version_names_in_indexes() {
        let program = parse(
            &lex("table metadata { id: Id primary auto type: String(32) version: Int required index { type version } }\n")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(program.tables[0].indexes[0].columns, ["type", "version"]);
    }

    #[test]
    fn rejects_decimal_precision_outside_runtime_limits() {
        for source in [
            "table values { id: Id primary auto amount: Decimal(40, 2) }",
            "table values { id: Id primary auto amount: Decimal(8, 10) }",
            "table values { id: Id primary auto amount: Decimal(2, 2) }",
            "table values { id: Id primary auto amount: Decimal(8) }",
        ] {
            assert!(parse(&lex(source).unwrap()).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn rejects_unknown_foreign_key_delete_actions() {
        let source = "table customers { id: Id primary auto } table invoices { customer: Customer on_delete truncate }";
        let error = parse(&lex(source).unwrap()).unwrap_err();
        assert!(error.message.contains("unknown on-delete action"));
    }

    #[test]
    fn parses_explicit_and_schema_mapped_form() {
        let source = r#"
            form CustomerForm {
                field email: Email {
                    label: "E-Mail"
                    required
                    min_length: 2
                    max: 255
                    widget: email
                }
                action save {
                    requires auth
                    permits "customers.save"
                    sql {
                        INSERT INTO customers (email) VALUES (:email)
                    }
                    success "Saved"
                    redirect "/customers"
                }
            }
            form CustomerCreate -> customers {
                route: "/customers/new"
                fields {
                    name
                    active
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.forms.len(), 2);
        assert_eq!(program.forms[0].fields[0].widget.as_deref(), Some("email"));
        assert_eq!(program.forms[0].fields[0].min_length, Some(2));
        assert_eq!(
            program.forms[0].actions[0].redirect.as_deref(),
            Some("/customers")
        );
        assert!(program.forms[0].actions[0].requires_auth);
        assert_eq!(program.forms[0].actions[0].permissions, ["customers.save"]);
        assert!(matches!(
            program.forms[0].actions[0].statements[0],
            Stmt::Expr(Expr {
                kind: ExprKind::Sql { .. },
                ..
            })
        ));
        assert_eq!(program.forms[1].table.as_deref(), Some("customers"));
        assert_eq!(program.forms[1].path.as_deref(), Some("/customers/new"));
        assert_eq!(program.forms[1].fields.len(), 2);
    }

    #[test]
    fn parses_bounded_file_upload_form_metadata() {
        let source = r#"
            form InvoiceDocument {
                field document: File {
                    label: "Rechnungs-PDF"
                    required
                    max_size: "10MB"
                    allowed_types: ["application/pdf", "image/png"]
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let field = &program.forms[0].fields[0];
        assert_eq!(field.ty, Some(Type::Named("File".into())));
        let upload = field.file_upload.as_ref().unwrap();
        assert_eq!(upload.max_size_bytes, 10_000_000);
        assert_eq!(upload.allowed_types, ["application/pdf", "image/png"]);
    }

    #[test]
    fn rejects_unsupported_file_size_units() {
        let source = r#"form Upload { field document: File { max_size: "10GB" allowed_types: ["application/pdf"] } }"#;
        let error = parse(&lex(source).unwrap()).unwrap_err();
        assert!(error
            .message
            .contains("max_size must be a positive byte size"));
    }

    #[test]
    fn rejects_duplicate_file_upload_options() {
        for source in [
            r#"form Upload { field document: File { max_size: "1MB" max_size: "2MB" allowed_types: ["application/pdf"] } }"#,
            r#"form Upload { field document: File { max_size: "1MB" allowed_types: ["application/pdf"] allowed_types: ["image/png"] } }"#,
        ] {
            assert!(parse(&lex(source).unwrap()).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn parses_exact_numeric_form_bounds() {
        let source = r#"
            form Invoice {
                field total: Decimal {
                    min_value: "0.01"
                    max_value: "9999.99"
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let field = &program.forms[0].fields[0];
        assert_eq!(field.min_value.as_deref(), Some("0.01"));
        assert_eq!(field.max_value.as_deref(), Some("9999.99"));
    }

    #[test]
    fn parses_static_form_choice_lists() {
        let source = r#"
            form Invoice {
                field status: String {
                    required
                    choices: ["draft", "issued", "paid"]
                }
                field type: String {
                    choices: ["invoice", "quote"]
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(
            program.forms[0].fields[0].choices,
            ["draft", "issued", "paid"]
        );
        assert_eq!(program.forms[0].fields[1].name, "type");
        let empty = r#"form Invalid { field status: String { choices: [] } }"#;
        assert!(parse(&lex(empty).unwrap())
            .unwrap_err()
            .message
            .contains("must contain at least one value"));
    }

    #[test]
    fn parses_minimal_crud_definition() {
        let program = parse(&lex("crud Machine -> machines").unwrap()).unwrap();
        assert_eq!(program.cruds.len(), 1);
        assert_eq!(program.cruds[0].name, "Machine");
        assert_eq!(program.cruds[0].table, "machines");
    }

    #[test]
    fn parses_typed_crud_child_collection_and_aggregates() {
        let source = r#"
            crud Invoice -> invoices {
                has_many items: InvoiceItem {
                    foreign_key: invoice_id
                    order_by: position asc
                    cascade: delete
                    aggregates {
                        net_amount: sum(total_net)
                        tax_amount: sum(total_gross - total_net)
                    }
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let child = &program.cruds[0].children[0];
        assert_eq!(child.name, "items");
        assert_eq!(child.model, "InvoiceItem");
        assert_eq!(child.foreign_key, "invoice_id");
        assert_eq!(child.order_by.as_deref(), Some("position"));
        assert!(!child.descending);
        assert_eq!(
            child.on_delete,
            Some(zelyra_ast::ReferentialAction::Cascade)
        );
        assert_eq!(child.aggregates.len(), 2);
        assert_eq!(child.aggregates[1].target, "tax_amount");
        assert!(child.span.start < child.span.end);
    }

    #[test]
    fn rejects_duplicate_crud_child_and_aggregate_declarations() {
        let duplicate_child = r#"crud Invoice -> invoices {
            has_many items: InvoiceItem { foreign_key: invoice_id }
            has_many items: InvoiceItem { foreign_key: invoice_id }
        }"#;
        assert!(parse(&lex(duplicate_child).unwrap())
            .unwrap_err()
            .message
            .contains("duplicate CRUD child relation"));

        let duplicate_aggregate = r#"crud Invoice -> invoices {
            has_many items: InvoiceItem {
                foreign_key: invoice_id
                aggregates { net_amount: sum(total_net) net_amount: sum(total_gross) }
            }
        }"#;
        assert!(parse(&lex(duplicate_aggregate).unwrap())
            .unwrap_err()
            .message
            .contains("duplicate aggregate target"));
    }

    #[test]
    fn parses_tenant_membership_and_crud_scope_declarations() {
        let program = parse(
            &lex(r#"auth users {
                    table: users
                    sessions: auth_sessions
                    memberships: tenant_memberships
                    tenant_owner: tenants.user_id
                }
                crud Invoice -> invoices {
                    tenant: tenant_id
                    requires auth
                }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.auth[0].membership_table.as_deref(),
            Some("tenant_memberships")
        );
        assert_eq!(
            program.auth[0].tenant_owner_table.as_deref(),
            Some("tenants")
        );
        assert_eq!(
            program.auth[0].tenant_owner_column.as_deref(),
            Some("user_id")
        );
        assert_eq!(program.cruds[0].tenant_column.as_deref(), Some("tenant_id"));
    }

    #[test]
    fn parses_configured_crud_definition() {
        let program = parse(
            &lex(r#"crud Customer -> customers {
                    title: "Customers"
                    unread: read_at
                    expires: expires_at
                    soft_delete { column: deleted_at }
                    list { customer_number name }
                    search { name }
                    filter { active }
                }"#)
            .unwrap(),
        )
        .unwrap();
        let crud = &program.cruds[0];
        assert_eq!(crud.title.as_deref(), Some("Customers"));
        assert_eq!(crud.unread_column.as_deref(), Some("read_at"));
        assert_eq!(crud.expires_column.as_deref(), Some("expires_at"));
        assert_eq!(
            crud.soft_delete
                .as_ref()
                .map(|config| config.column.as_str()),
            Some("deleted_at")
        );
        assert_eq!(crud.list, ["customer_number", "name"]);
        assert_eq!(crud.search, ["name"]);
        assert_eq!(crud.filters, ["active"]);
    }

    #[test]
    fn parses_crud_layout_view_reference() {
        let program = parse(
            &lex(r#"crud Customer -> customers {
                    layout: AppShell
                    slots {
                        header {
                            html {
                                <h1>Customers</h1>
                            }
                        }
                    }
                }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.cruds[0].layout.as_deref(), Some("AppShell"));
        assert_eq!(program.cruds[0].layout_slots.len(), 1);
        assert_eq!(program.cruds[0].layout_slots[0].name, "header");
        assert_eq!(
            program.cruds[0].layout_slots[0].html.trim(),
            "<h1>Customers</h1>"
        );
    }

    #[test]
    fn parses_crud_list_view_override() {
        let program = parse(
            &lex(r#"crud Customer -> customers {
                    view {
                        list {
                            mode: cards
                            empty: "No customers yet."
                        }
                        detail {
                            mode: cards
                            title: "Customer details"
                        }
                        form {
                            mode: cards
                            title: "Edit customer"
                            submit: "Save customer"
                        }
                        delete {
                            title: "Delete customer"
                            message: "This cannot be undone."
                            submit: "Delete now"
                        }
                        loading {
                            message: "Loading customer..."
                        }
                        error {
                            title: "Customer unavailable"
                            message: "Please try again later."
                        }
                    }
                    action deactivate {
                        label: "Deactivate customer"
                        icon: "pause"
                        confirm: "Deactivate this customer?"
                        confirm_page {
                            title: "Confirm deactivation"
                            message: "This cannot be undone."
                            submit: "Deactivate now"
                        }
                        field active: Bool { required }
                        permits "customers.edit"
                        sql {
                            UPDATE customers
                            SET active = false
                            WHERE id = :id
                        }
                        success "Customer deactivated."
                        success_page {
                            title: "Customer updated"
                            message: "The customer was updated."
                        }
                        error_page {
                            title: "Customer action failed"
                            message: "The customer could not be updated."
                        }
                        redirect "/customers"
                    }
                }"#)
            .unwrap(),
        )
        .unwrap();
        let view = &program.cruds[0].view.list;
        assert_eq!(view.mode, CrudListViewMode::Cards);
        assert_eq!(view.empty.as_deref(), Some("No customers yet."));
        assert_eq!(program.cruds[0].view.detail.mode, CrudDetailViewMode::Cards);
        assert_eq!(
            program.cruds[0].view.detail.title.as_deref(),
            Some("Customer details")
        );
        assert_eq!(program.cruds[0].view.form.mode, CrudFormViewMode::Cards);
        assert_eq!(
            program.cruds[0].view.form.title.as_deref(),
            Some("Edit customer")
        );
        assert_eq!(
            program.cruds[0].view.form.submit.as_deref(),
            Some("Save customer")
        );
        assert_eq!(
            program.cruds[0].view.delete.title.as_deref(),
            Some("Delete customer")
        );
        assert_eq!(
            program.cruds[0].view.delete.message.as_deref(),
            Some("This cannot be undone.")
        );
        assert_eq!(
            program.cruds[0].view.delete.submit.as_deref(),
            Some("Delete now")
        );
        assert_eq!(
            program.cruds[0].view.loading.message.as_deref(),
            Some("Loading customer...")
        );
        assert_eq!(
            program.cruds[0].view.error.title.as_deref(),
            Some("Customer unavailable")
        );
        assert_eq!(
            program.cruds[0].view.error.message.as_deref(),
            Some("Please try again later.")
        );
        assert_eq!(program.cruds[0].actions.len(), 1);
        assert_eq!(program.cruds[0].actions[0].name, "deactivate");
        assert_eq!(
            program.cruds[0].actions[0].label.as_deref(),
            Some("Deactivate customer")
        );
        assert_eq!(program.cruds[0].actions[0].icon.as_deref(), Some("pause"));
        assert_eq!(
            program.cruds[0].actions[0].confirm.as_deref(),
            Some("Deactivate this customer?")
        );
        assert_eq!(
            program.cruds[0].actions[0]
                .confirm_page
                .as_ref()
                .and_then(|view| view.title.as_deref()),
            Some("Confirm deactivation")
        );
        assert_eq!(
            program.cruds[0].actions[0]
                .confirm_page
                .as_ref()
                .and_then(|view| view.message.as_deref()),
            Some("This cannot be undone.")
        );
        assert_eq!(
            program.cruds[0].actions[0]
                .confirm_page
                .as_ref()
                .and_then(|view| view.submit.as_deref()),
            Some("Deactivate now")
        );
        assert_eq!(program.cruds[0].actions[0].fields.len(), 1);
        assert_eq!(program.cruds[0].actions[0].fields[0].name, "active");
        assert_eq!(program.cruds[0].actions[0].permissions, ["customers.edit"]);
        assert_eq!(
            program.cruds[0].actions[0].success.as_deref(),
            Some("Customer deactivated.")
        );
        assert_eq!(
            program.cruds[0].actions[0]
                .success_page
                .as_ref()
                .and_then(|page| page.title.as_deref()),
            Some("Customer updated")
        );
        assert_eq!(
            program.cruds[0].actions[0]
                .error_page
                .as_ref()
                .and_then(|page| page.message.as_deref()),
            Some("The customer could not be updated.")
        );
    }

    #[test]
    fn parses_separate_crud_create_edit_content_and_tableview_row_actions() {
        let source = r#"
            crud Customer -> customers {
                view {
                    form { title: "Customer" submit: "Save" }
                    create {
                        title: "New customer"
                        before_form { html { <p>Before</p> } }
                        field_content {
                            email { html { <small>Contact: {values.email}</small> } }
                        }
                    }
                    edit {
                        submit: "Update"
                        after_form { html { <p>{record.name}</p> } }
                    }
                }
            }
            tableview Customers {
                source sql<Customer[]> { SELECT id, name, email FROM customers }
                columns { name }
                view {
                    before_content { html { <p>Heading</p> } }
                    row_fields { id email }
                    column_content {
                        item: customer
                        name { html { <small>{customer.email}</small> } }
                    }
                    row_actions {
                        item: customer
                        html { <a href="/customers/{customer.id}">Open</a> }
                    }
                    after_content { html { <p>Footer</p> } }
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let crud = &program.cruds[0].view;
        assert_eq!(crud.form.title.as_deref(), Some("Customer"));
        assert_eq!(crud.create.title.as_deref(), Some("New customer"));
        assert_eq!(crud.create.field_content[0].field, "email");
        assert_eq!(crud.edit.submit.as_deref(), Some("Update"));
        assert!(crud
            .create
            .before_form
            .as_deref()
            .unwrap()
            .contains("Before"));
        assert!(crud
            .edit
            .after_form
            .as_deref()
            .unwrap()
            .contains("record.name"));
        let tableview = &program.tableviews[0];
        assert!(tableview
            .view
            .before_content
            .as_deref()
            .unwrap()
            .contains("Heading"));
        assert_eq!(
            tableview.view.row_actions.as_ref().unwrap().item,
            "customer"
        );
        assert_eq!(tableview.view.row_fields, ["id", "email"]);
        assert_eq!(tableview.view.column_content[0].column, "name");
        assert_eq!(tableview.view.column_content[0].item, "customer");
        assert!(tableview.view.column_content[0]
            .html
            .contains("customer.email"));
        assert!(tableview
            .view
            .after_content
            .as_deref()
            .unwrap()
            .contains("Footer"));
    }

    #[test]
    fn parses_policy_scoped_tableview_row_actions() {
        let source = r#"
            crud Invoice -> invoices {
                guard edit when locked == true { error: "Locked" }
                guard delete when locked == true { error: "Locked" }
            }
            tableview InvoiceRows {
                source sql<Invoice[]> { SELECT id, status, locked FROM invoices }
                columns { status }
                view {
                    row_fields { id locked }
                    row_actions {
                        item: invoice
                        crud: Invoice
                        edit { html { <a href="/invoices/{invoice.id}/edit">Edit</a> } }
                        delete { html { <button type="button">Delete</button> } }
                    }
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let actions = program.tableviews[0].view.row_actions.as_ref().unwrap();
        assert_eq!(actions.crud_model.as_deref(), Some("Invoice"));
        assert!(actions.html.is_empty());
        assert!(actions.edit_html.as_deref().unwrap().contains("/edit"));
        assert!(actions.delete_html.as_deref().unwrap().contains("Delete"));
    }

    #[test]
    fn rejects_duplicate_crud_field_content() {
        let source = r#"crud Customer -> customers { view { create {
            field_content { email { html { <p>A</p> } } email { html { <p>B</p> } } }
        } } }"#;
        let error = parse(&lex(source).unwrap()).unwrap_err();
        assert!(error.message.contains("duplicate field content"));
    }

    #[test]
    fn parses_shared_crud_view_fields() {
        let program = parse(
            &lex(r#"table customers {
                    id: Id primary auto
                    name: String(100) required
                    email: Email?
                    active: Bool default true
                }
                crud Customer -> customers {
                    view {
                        fields { name email active }
                    }
                }"#)
            .unwrap(),
        )
        .unwrap();

        assert_eq!(program.cruds[0].view.fields, ["name", "email", "active"]);
        assert!(program.cruds[0].list.is_empty());
    }

    #[test]
    fn rejects_unknown_crud_list_view_mode() {
        let result = parse(
            &lex(r#"crud Customer -> customers {
                    view { list { mode: carousel } }
                }"#)
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_crud_detail_view_mode() {
        let result = parse(
            &lex(r#"crud Customer -> customers {
                    view { detail { mode: full } }
                }"#)
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_crud_form_view_mode() {
        let result = parse(
            &lex(r#"crud Customer -> customers {
                    view { form { mode: wizard } }
                }"#)
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_crud_delete_view_property() {
        let result = parse(
            &lex(r#"crud Customer -> customers {
                    view { delete { confirm: "yes" } }
                }"#)
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_crud_error_view_property() {
        let result = parse(
            &lex(r#"crud Customer -> customers {
                    view { error { retry: "yes" } }
                }"#)
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn parses_auth_and_protected_crud_definition() {
        let program = parse(
            &lex(r#"auth users {
                    table: users
                    sessions: auth_sessions
                    permissions: user_permissions
                    roles: user_roles
                    role_permissions: role_permissions
                    audit: auth_audit_log
                    audit_chain: true
                    admin_path: "/admin/access"
                    admin_permission: "auth.manage"
                    admin_role: admin
                }

                crud Customer -> customers {
                    requires auth
                    permits "customers.view"
                    permits create "customers.create"
                    permits edit "customers.edit"
                    permits delete "customers.delete"
                }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.auth[0].name, "users");
        assert_eq!(program.auth[0].table, "users");
        assert_eq!(
            program.auth[0].session_table.as_deref(),
            Some("auth_sessions")
        );
        assert_eq!(
            program.auth[0].permissions_table.as_deref(),
            Some("user_permissions")
        );
        assert_eq!(program.auth[0].roles_table.as_deref(), Some("user_roles"));
        assert_eq!(
            program.auth[0].role_permissions_table.as_deref(),
            Some("role_permissions")
        );
        assert_eq!(
            program.auth[0].audit_table.as_deref(),
            Some("auth_audit_log")
        );
        assert!(program.auth[0].audit_chain);
        assert_eq!(program.auth[0].admin_path.as_deref(), Some("/admin/access"));
        assert_eq!(
            program.auth[0].admin_permission.as_deref(),
            Some("auth.manage")
        );
        assert_eq!(program.auth[0].admin_role.as_deref(), Some("admin"));
        assert!(program.cruds[0].requires_auth);
        assert_eq!(program.cruds[0].permissions, ["customers.view"]);
        assert_eq!(program.cruds[0].create_permissions, ["customers.create"]);
        assert_eq!(program.cruds[0].edit_permissions, ["customers.edit"]);
        assert_eq!(program.cruds[0].delete_permissions, ["customers.delete"]);
    }

    #[test]
    fn keeps_unscoped_crud_permission_as_legacy_default() {
        let program = parse(
            &lex(r#"crud Customer -> customers {
                    permits view "customers.view"
                }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.cruds[0].permissions, ["customers.view"]);
        assert!(program.cruds[0].create_permissions.is_empty());
        assert!(program.cruds[0].edit_permissions.is_empty());
        assert!(program.cruds[0].delete_permissions.is_empty());
    }

    #[test]
    fn parses_row_conditioned_crud_permissions_and_lifecycle_guards() {
        let program = parse(
            &lex(r#"
                crud Invoice -> invoices {
                    permits edit "invoices.manage" when status == "draft"
                    permits delete "invoices.manage" when editable == true
                    guard edit when locked == true { error: "Locked invoices cannot be edited." }
                    guard delete when locked == true { error: "Locked invoices cannot be deleted." }
                }
            "#)
            .unwrap(),
        )
        .unwrap();
        let crud = &program.cruds[0];
        assert_eq!(crud.conditional_permissions.len(), 2);
        assert_eq!(
            crud.conditional_permissions[0].action,
            CrudRecordAction::Edit
        );
        assert_eq!(
            crud.conditional_permissions[0].permission,
            "invoices.manage"
        );
        assert_eq!(crud.guards.len(), 2);
        assert_eq!(crud.guards[0].action, CrudRecordAction::Edit);
        assert_eq!(crud.guards[1].message, "Locked invoices cannot be deleted.");
    }

    #[test]
    fn parses_typed_api_definition() {
        let program = parse(
            &lex(r#"
                api GET "/customers/{id}" {
                    handler get_customer
                    input {
                        id: CustomerId
                    }
                    output Customer
                    errors {
                        403 Forbidden
                        404 NotFound
                    }
                }
                fn main() { }
            "#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.apis.len(), 1);
        assert_eq!(program.apis[0].method, "GET");
        assert_eq!(program.apis[0].path, "/customers/{id}");
        assert_eq!(program.apis[0].handler.as_deref(), Some("get_customer"));
        assert!(!program.apis[0].requires_auth);
        assert_eq!(program.apis[0].input[0].name, "id");
        assert_eq!(program.apis[0].errors[1].status, 404);
    }

    #[test]
    fn parses_api_table_validation_binding() {
        let program = parse(
            &lex(r#"api POST "/invoices" { validates invoices input { amount: Int } output Result<Int, String> errors { 422 ValidationError: String } } fn main() { }"#).unwrap(),
        ).unwrap();
        assert_eq!(program.apis[0].validates_table.as_deref(), Some("invoices"));
    }

    #[test]
    fn parses_api_authentication_and_permissions() {
        let program = parse(
            &lex(r#"api GET "/customers" {
                    requires auth
                    permits "customers.view"
                    output String
                }
                fn main() { }"#)
            .unwrap(),
        )
        .unwrap();
        assert!(program.apis[0].requires_auth);
        assert_eq!(program.apis[0].permissions, ["customers.view"]);
    }

    #[test]
    fn parses_api_version_deprecation_and_bounded_rate_limit() {
        let program = parse(
            &lex(r#"api GET "/v1/customers" {
                    version "v1"
                    deprecated
                    rate_limit 30 per 60
                    output String
                }
                fn main() { }"#)
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        assert_eq!(api.version.as_deref(), Some("v1"));
        assert!(api.deprecated);
        assert_eq!(
            api.rate_limit,
            Some(zelyra_ast::ApiRateLimit {
                requests: 30,
                window_seconds: 60,
            })
        );
    }

    #[test]
    fn parses_configurable_login_rate_limit_and_block_duration() {
        let program = parse(
            &lex(r#"auth users {
                    table: users
                    login_rate_limit: 3 per 120
                    login_block_seconds: 45
                }
                fn main() { }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.auth[0].login_rate_limit,
            Some(zelyra_ast::ApiRateLimit {
                requests: 3,
                window_seconds: 120,
            })
        );
        assert_eq!(program.auth[0].login_block_seconds, Some(45));
    }

    #[test]
    fn parses_configurable_password_reset_table_and_rate_limit() {
        let program = parse(
            &lex(r#"auth users {
                    table: users
                    reset_tokens: password_resets
                    reset_rate_limits: reset_limits
                    reset_rate_limit: 3 per 900
                    reset_block_seconds: 600
                }
                fn main() { }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.auth[0].reset_tokens_table.as_deref(),
            Some("password_resets")
        );
        assert_eq!(
            program.auth[0].reset_rate_limits_table.as_deref(),
            Some("reset_limits")
        );
        assert_eq!(
            program.auth[0].reset_rate_limit,
            Some(zelyra_ast::ApiRateLimit {
                requests: 3,
                window_seconds: 900,
            })
        );
        assert_eq!(program.auth[0].reset_block_seconds, Some(600));
    }

    #[test]
    fn rejects_out_of_bounds_or_duplicate_login_rate_settings() {
        for source in [
            r#"auth users { table: users login_rate_limit: 0 per 60 } fn main() { }"#,
            r#"auth users { table: users login_rate_limit: 5 per 0 } fn main() { }"#,
            r#"auth users { table: users login_block_seconds: 86401 } fn main() { }"#,
            r#"auth users { table: users login_rate_limit: 5 per 60 login_rate_limit: 3 per 30 } fn main() { }"#,
        ] {
            assert!(parse(&lex(source).unwrap()).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn rejects_out_of_bounds_or_duplicate_reset_rate_settings() {
        for source in [
            r#"auth users { table: users reset_rate_limit: 0 per 60 } fn main() { }"#,
            r#"auth users { table: users reset_rate_limit: 5 per 0 } fn main() { }"#,
            r#"auth users { table: users reset_block_seconds: 86401 } fn main() { }"#,
            r#"auth users { table: users reset_rate_limit: 5 per 60 reset_rate_limit: 3 per 30 } fn main() { }"#,
            r#"auth users { table: users reset_rate_limits: first reset_rate_limits: second } fn main() { }"#,
        ] {
            assert!(parse(&lex(source).unwrap()).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn rejects_out_of_bounds_api_rate_limits_and_invalid_versions() {
        for source in [
            r#"api GET "/limited" { rate_limit 0 per 60 output String } fn main() { }"#,
            r#"api GET "/limited" { rate_limit 10 per 86401 output String } fn main() { }"#,
            r#"api GET "/versioned" { version "v1\nInjected" output String } fn main() { }"#,
        ] {
            assert!(parse(&lex(source).unwrap()).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn parses_structured_record_definition() {
        let program = parse(
            &lex(r#"struct CustomerInput {
                    name: String
                    email: Email?
                }
                fn main() { }"#)
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.records.len(), 1);
        assert_eq!(program.records[0].name, "CustomerInput");
        assert_eq!(
            program.records[0].fields[1].ty,
            Type::Option(Box::new(Type::Named("Email".into())))
        );
    }

    #[test]
    fn parses_generic_record_definitions_types_and_explicit_literals() {
        let program = parse(
            &lex("struct Box<T> { value: T } fn unwrap(boxed: Box<Int>) -> Int { return boxed.value } fn main() { boxed: Box<Int> = Box<Int> { value: 7 } print(unwrap(boxed)) }")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(program.records[0].type_params, ["T"]);
        assert!(matches!(
            program.functions[0].params[0].ty,
            Type::Applied { ref name, ref args }
                if name == "Box" && args == &[Type::Int]
        ));
        let value = match &program.functions[1].body.statements[0] {
            Stmt::BindOrAssign { value, .. } | Stmt::Let { value, .. } => value,
            _ => panic!("expected typed generic record binding"),
        };
        assert!(matches!(
            value.kind,
            ExprKind::Record { ref type_name, ref type_args, .. }
                if type_name == "Box" && type_args == &[Type::Int]
        ));
    }

    #[test]
    fn parses_array_literals_and_indexing() {
        let program =
            parse(&lex("fn main() { values = [1, 2, 3] print(values[1]) }").unwrap()).unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected array binding");
        };
        assert!(matches!(value.kind, ExprKind::Array(_)));
        let Stmt::Expr(expression) = &program.functions[0].body.statements[1] else {
            panic!("expected print expression");
        };
        let ExprKind::Call { args, .. } = &expression.kind else {
            panic!("expected print call");
        };
        assert!(matches!(args[0].kind, ExprKind::Index { .. }));
    }

    #[test]
    fn parses_record_literals_and_field_access() {
        let program = parse(
            &lex("struct Address { city: String } fn main() { address = Address { city: \"Berlin\" } print(address.city) }").unwrap(),
        )
        .unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected record binding");
        };
        assert!(matches!(value.kind, ExprKind::Record { .. }));
        let Stmt::Expr(expression) = &program.functions[0].body.statements[1] else {
            panic!("expected print expression");
        };
        let ExprKind::Call { args, .. } = &expression.kind else {
            panic!("expected print call");
        };
        assert!(matches!(args[0].kind, ExprKind::Field { .. }));
    }

    #[test]
    fn parses_typed_function_calls() {
        let program =
            parse(&lex("fn main() { value = json_decode<Customer>(body) }").unwrap()).unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected binding");
        };
        let ExprKind::Call {
            name, type_args, ..
        } = &value.kind
        else {
            panic!("expected typed call");
        };
        assert_eq!(name, "json_decode");
        assert_eq!(type_args, &[Type::Named("Customer".into())]);
    }

    #[test]
    fn parses_result_typed_json_calls() {
        let program =
            parse(&lex("fn main() { value = json_decode_result<Customer>(body) }").unwrap())
                .unwrap();
        let Stmt::BindOrAssign { value, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected binding");
        };
        let ExprKind::Call {
            name, type_args, ..
        } = &value.kind
        else {
            panic!("expected typed call");
        };
        assert_eq!(name, "json_decode_result");
        assert_eq!(type_args, &[Type::Named("Customer".into())]);
    }

    #[test]
    fn parses_array_for_loops() {
        let program =
            parse(&lex("fn main() { for value in [1, 2] { print(value) } }").unwrap()).unwrap();
        assert!(matches!(
            program.functions[0].body.statements[0],
            Stmt::For { ref name, .. } if name == "value"
        ));
    }

    #[test]
    fn parses_tuple_destructuring_for_loops() {
        let program = parse(
            &lex("fn main() { for (key, value) in entries(Map { \"a\": 1 }) { print(key) } }")
                .unwrap(),
        )
        .unwrap();
        let Stmt::For {
            name,
            additional_names,
            ..
        } = &program.functions[0].body.statements[0]
        else {
            panic!("expected a for loop");
        };
        assert_eq!(name, "key");
        assert_eq!(additional_names, &["value"]);

        let duplicate =
            parse(&lex("fn main() { for (item, item) in zip([1], [2]) { print(item) } }").unwrap())
                .unwrap_err();
        assert!(duplicate.message.contains("duplicate loop variable"));
    }

    #[test]
    fn parses_stepped_integer_for_loops_as_three_argument_ranges() {
        let program =
            parse(&lex("fn main() { for value in 1..21 step 2 { print(value) } }").unwrap())
                .unwrap();
        let Stmt::For { iterable, .. } = &program.functions[0].body.statements[0] else {
            panic!("expected integer range loop");
        };
        let ExprKind::Call { name, args, .. } = &iterable.kind else {
            panic!("expected range call");
        };
        assert_eq!(name, "range");
        assert_eq!(args.len(), 3);
        assert!(matches!(args[2].kind, ExprKind::Int(2)));
    }

    #[test]
    fn rejects_step_on_non_range_for_loops() {
        let error =
            parse(&lex("fn main() { for value in [1, 2] step 2 { print(value) } }").unwrap())
                .expect_err("step must only follow an integer range");
        assert!(error
            .message
            .contains("`step` is only supported for integer ranges"));
    }

    #[test]
    fn parses_map_literals_and_types() {
        let program = parse(
            &lex("fn lookup(values: Map<String, Int>) -> Map<String, Int> { return values } fn main() { values: Map<String, Int> = Map { \"one\": 1, \"two\": 2 } print(get(values, \"one\")) }").unwrap(),
        )
        .unwrap();
        assert_eq!(
            program.functions[0].params[0].ty,
            Type::Map(Box::new(Type::String), Box::new(Type::Int))
        );
        let Stmt::Let { value, ty, .. } = &program.functions[1].body.statements[0] else {
            panic!("expected typed Map binding");
        };
        assert_eq!(
            ty,
            &Some(Type::Map(Box::new(Type::String), Box::new(Type::Int)))
        );
        assert!(matches!(value.kind, ExprKind::Map(_)));
    }

    #[test]
    fn parses_parallel_await_bindings() {
        let program = parse(
            &lex("fn load() -> Int { return 1 } fn main() { parallel { value = await load() } print(value) }")
                .unwrap(),
        )
        .unwrap();
        let Stmt::Parallel { body, .. } = &program.functions[1].body.statements[0] else {
            panic!("expected parallel statement");
        };
        assert!(matches!(
            body.statements[0],
            Stmt::BindOrAssign {
                value: Expr {
                    kind: ExprKind::Await(_),
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn parses_function_capabilities() {
        let program = parse(
            &lex(
                "fn send_invoice(order: Order) -> Unit\n uses Network, FileSystem\n { return } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.functions[0].capabilities, ["Network", "FileSystem"]);
    }

    #[test]
    fn parses_scoped_database_capabilities() {
        let program = parse(
            &lex("fn report() uses Database(read) { rows = sql<Int> { SELECT 1 } } fn main() { }")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(program.functions[0].capabilities, ["Database(read)"]);
    }

    #[test]
    fn parses_function_contracts() {
        let program = parse(
            &lex(
                "fn increment(value: Int) -> Int\n requires { value >= 0 }\n ensures { result > value }\n { return value + 1 } fn main() { print(increment(1)) }",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.functions[0].requires.len(), 1);
        assert_eq!(program.functions[0].ensures.len(), 1);
    }

    #[test]
    fn parses_project_relative_import_and_public_function() {
        let program = parse(
            &lex("import \"src/math.zyl\" as math\npub fn add(a: Int, b: Int) -> Int { return a + b }\nfn main() { print(math::add(2, 3)) }").unwrap(),
        )
        .unwrap();

        assert_eq!(program.imports.len(), 1);
        assert_eq!(program.imports[0].path, "src/math.zyl");
        assert_eq!(program.imports[0].alias, "math");
        assert!(program.functions[0].is_public);
        let Stmt::Expr(expression) = &program.functions[1].body.statements[0] else {
            panic!("expected print expression");
        };
        let ExprKind::Call { args, .. } = &expression.kind else {
            panic!("expected print call");
        };
        assert!(matches!(
            &args[0].kind,
            ExprKind::Call { name, .. } if name == "math::add"
        ));
    }

    #[test]
    fn parses_public_views_and_components() {
        let program = parse(
            &lex(
                "pub view Shell { html { <main><slot /></main> } }\npub component Badge { html { <strong>Badge</strong> } }",
            )
            .unwrap(),
        )
        .unwrap();

        assert!(program.views[0].is_public);
        assert!(program.components[0].is_public);
    }

    #[test]
    fn rejects_imports_after_declarations_and_public_unsupported_declarations() {
        let late_import =
            parse(&lex("fn main() {}\nimport \"src/math.zyl\" as math").unwrap()).unwrap_err();
        assert!(late_import
            .message
            .contains("must appear before declarations"));

        let invalid_visibility =
            parse(&lex("pub table customers { id: Id primary auto }").unwrap()).unwrap_err();
        assert!(invalid_visibility
            .message
            .contains("functions, types, enums, records, classes, views, and components"));
    }

    #[test]
    fn parses_table_module_access_grants() {
        let program = parse(
            &lex(
                "table customers { id: Id primary auto access { read: [\"src/reports.zyl\"] write: [\"src/importer.zyl\"] read_write: [\"src/admin.zyl\"] } }",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.tables[0].access.read, ["src/reports.zyl"]);
        assert_eq!(program.tables[0].access.write, ["src/importer.zyl"]);
        assert_eq!(program.tables[0].access.read_write, ["src/admin.zyl"]);
        assert!(program.tables[0].access.span.is_some());
    }

    #[test]
    fn rejects_duplicate_table_access_modes_and_grants() {
        let duplicate_mode = parse(
            &lex("table customers { id: Id primary auto access { read: [] read: [] } }").unwrap(),
        )
        .unwrap_err();
        assert!(duplicate_mode.message.contains("declared more than once"));

        let duplicate_grant = parse(
            &lex("table customers { id: Id primary auto access { write: [\"src/admin.zyl\", \"src/admin.zyl\"] } }")
                .unwrap(),
        )
        .unwrap_err();
        assert!(duplicate_grant.message.contains("is repeated"));
    }

    #[test]
    fn parses_public_records_aliases_and_qualified_types() {
        let program = parse(
            &lex("pub type CustomerId = Int\npub struct Customer { id: CustomerId }\nfn load(id: crm::CustomerId) -> crm::Customer { return crm::Customer { id: id } }").unwrap(),
        )
        .unwrap();

        assert!(program.types[0].is_public);
        assert!(program.records[0].is_public);
        assert_eq!(
            program.functions[0].params[0].ty,
            Type::Named("crm::CustomerId".into())
        );
        assert_eq!(
            program.functions[0].return_type,
            Some(Type::Named("crm::Customer".into()))
        );
        let Stmt::Return {
            value: Some(value), ..
        } = &program.functions[0].body.statements[0]
        else {
            panic!("expected returned record literal");
        };
        assert!(
            matches!(&value.kind, ExprKind::Record { type_name, .. } if type_name == "crm::Customer")
        );
    }

    #[test]
    fn parses_typed_api_error_payloads() {
        let program = parse(
            &lex(
                r#"struct Problem { message: String } api GET "/fail" { output Result<String, Problem> errors { 422 Validation: Problem } } fn main() { }"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.apis[0].errors[0].name, "Validation");
        assert_eq!(
            program.apis[0].errors[0].payload,
            Some(Type::Named("Problem".into()))
        );
    }

    #[test]
    fn parses_native_command_and_host_schedule_declarations() {
        let source = r#"
command process_recurring_invoices {
    handler: run_recurring_invoices
    description: "Create invoices for active subscriptions"
}

schedule recurring_invoices {
    command: process_recurring_invoices
    cron: "0 4 * * *"
    timezone: "Europe/Berlin"
}

fn run_recurring_invoices() { }
"#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.commands.len(), 1);
        assert_eq!(program.commands[0].name, "process_recurring_invoices");
        assert_eq!(program.commands[0].handler, "run_recurring_invoices");
        assert_eq!(
            program.commands[0].description.as_deref(),
            Some("Create invoices for active subscriptions")
        );
        assert_eq!(program.schedules.len(), 1);
        assert_eq!(program.schedules[0].command, "process_recurring_invoices");
        assert_eq!(program.schedules[0].cron, "0 4 * * *");
        assert_eq!(
            program.schedules[0].timezone.as_deref(),
            Some("Europe/Berlin")
        );
    }

    #[test]
    fn rejects_unknown_or_missing_command_and_schedule_properties() {
        let unknown =
            parse(&lex("command sync { handler: run extra: true }").unwrap()).unwrap_err();
        assert!(unknown.message.contains("unknown command property"));

        let missing_handler =
            parse(&lex("command sync { description: \"Sync\" }").unwrap()).unwrap_err();
        assert!(missing_handler.message.contains("requires `handler`"));

        let missing_cron =
            parse(&lex("schedule nightly { command: sync timezone: \"UTC\" }").unwrap())
                .unwrap_err();
        assert!(missing_cron.message.contains("requires `cron`"));
    }

    #[test]
    fn parses_explicit_crud_action_execute_body_and_guard() {
        let source = r#"
            table invoices { id: Id primary auto is_locked: Bool required }
            crud Invoice -> invoices {
                action lock {
                    permits "invoices.lock"
                    uses Database(write), Clock
                    guard when row.is_locked == false {
                        error: "Invoice is already locked."
                    }
                    execute {
                        digest = hash_sha256("invoice")
                        sql { UPDATE invoices SET is_locked = true WHERE id = :id }
                    }
                }
            }
            fn main() { }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let action = &program.cruds[0].actions[0];
        assert_eq!(action.name, "lock");
        assert!(action.statements.is_empty());
        assert_eq!(action.permissions, ["invoices.lock"]);
        assert_eq!(action.capabilities, ["Database(write)", "Clock"]);
        assert_eq!(action.execute_body.as_ref().unwrap().statements.len(), 2);
        let guard = action.guard.as_ref().unwrap();
        assert_eq!(guard.message, "Invoice is already locked.");
        assert!(matches!(guard.condition.kind, ExprKind::Binary { .. }));
    }

    #[test]
    fn rejects_duplicate_or_mixed_crud_action_blocks() {
        let duplicate_guard = parse(
            &lex(r#"crud Invoice -> invoices { action lock { guard when true { error: "one" } guard when false { error: "two" } } } fn main() {}"#).unwrap(),
        )
        .unwrap_err();
        assert!(duplicate_guard.message.contains("only one `guard`"));

        let mixed = parse(
            &lex(r#"crud Invoice -> invoices { action lock { execute { value = 1 } value = 2 } } fn main() {}"#).unwrap(),
        )
        .unwrap_err();
        assert!(mixed
            .message
            .contains("cannot be mixed with an `execute` block"));
    }
}
