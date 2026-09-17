use std::fmt;
use zelyra_ast::*;
use zelyra_lexer::{Token, TokenKind};

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
    Parser { tokens, pos: 0 }.program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn current(&self) -> &'a Token {
        &self.tokens[self.pos]
    }
    fn at(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }
    fn advance(&mut self) -> &'a Token {
        let token = &self.tokens[self.pos];
        self.pos += 1;
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
            TokenKind::Ident(name) => {
                let span = self.advance().span;
                Ok((name.clone(), span))
            }
            _ => self.error(format!("expected {label}")),
        }
    }
    fn program(mut self) -> Result<Program, ParseError> {
        let mut databases = Vec::new();
        let mut tables = Vec::new();
        let mut types = Vec::new();
        let mut records = Vec::new();
        let mut pages = Vec::new();
        let mut forms = Vec::new();
        let mut cruds = Vec::new();
        let mut auth = Vec::new();
        let mut apis = Vec::new();
        let mut functions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Database) {
                databases.push(self.database_definition()?);
            } else if self.at(&TokenKind::Table) {
                tables.push(self.table_definition()?);
            } else if self.at(&TokenKind::Type) {
                types.push(self.type_definition()?);
            } else if self.at(&TokenKind::Struct) {
                records.push(self.record_definition()?);
            } else if self.at(&TokenKind::Page) {
                pages.push(self.page_definition()?);
            } else if self.at(&TokenKind::Form) {
                forms.push(self.form_definition()?);
            } else if self.at(&TokenKind::Crud) {
                cruds.push(self.crud_definition()?);
            } else if self.at(&TokenKind::Auth) {
                auth.push(self.auth_definition()?);
            } else if self.at(&TokenKind::Api) {
                apis.push(self.api_definition()?);
            } else {
                functions.push(self.function()?);
            }
            self.skip_newlines();
        }
        Ok(Program {
            databases,
            tables,
            types,
            records,
            pages,
            forms,
            cruds,
            auth,
            apis,
            functions,
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
        let mut output = None;
        let mut errors = Vec::new();
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
                    errors.push(ApiError { status, name, span });
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace, "`}` after API errors")?;
            } else {
                return self
                    .error("expected `handler`, `requires auth`, `permits`, `input`, `output`, or `errors` in API definition");
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
            output,
            errors,
            span: start.join(end),
        })
    }

    fn auth_definition(&mut self) -> Result<AuthDef, ParseError> {
        let start = self.expect(TokenKind::Auth, "auth")?;
        let (name, _) = self.ident("authentication name")?;
        self.expect(TokenKind::LBrace, "opening brace after authentication name")?;
        self.skip_newlines();
        let mut table = None;
        let mut session_table = None;
        let mut permissions_table = None;
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let field = match self.current().kind.clone() {
                TokenKind::Table => "table",
                TokenKind::Sessions => "sessions",
                TokenKind::Permissions => "permissions",
                _ => return self.error("expected table, sessions, or permissions option"),
            };
            self.advance();
            self.expect(TokenKind::Colon, "colon after authentication option")?;
            let value = self.ident("authentication option value")?.0;
            match field {
                "table" => table = Some(value),
                "sessions" => session_table = Some(value),
                "permissions" => permissions_table = Some(value),
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
            session_table,
            permissions_table,
            span: start.join(end),
        })
    }

    fn crud_definition(&mut self) -> Result<CrudDef, ParseError> {
        let start = self.expect(TokenKind::Crud, "`crud`")?;
        let (name, _) = self.ident("CRUD resource name")?;
        self.expect(TokenKind::Arrow, "`->` after CRUD resource name")?;
        let (table, table_span) = self.ident("table name after `->`")?;
        let mut title = None;
        let mut list = Vec::new();
        let mut search = Vec::new();
        let mut filters = Vec::new();
        let mut requires_auth = false;
        let mut permissions = Vec::new();

        if self.at(&TokenKind::LBrace) {
            self.advance();
            self.skip_newlines();
            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                match self.current().kind.clone() {
                    TokenKind::Title => {
                        self.advance();
                        self.expect(TokenKind::Colon, "colon after CRUD title")?;
                        title = Some(self.string_value("CRUD title")?);
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
                    TokenKind::Requires => {
                        self.advance();
                        self.expect(TokenKind::Auth, "auth after requires")?;
                        requires_auth = true;
                    }
                    TokenKind::Permits => {
                        self.advance();
                        permissions.push(self.string_value("permission")?);
                    }
                    _ => {
                        return self
                            .error("expected title, list, search, or filter in CRUD definition")
                    }
                }
                self.skip_newlines();
            }
            let end = self.expect(TokenKind::RBrace, "`}` after CRUD definition")?;
            return Ok(CrudDef {
                name,
                table,
                title,
                list,
                search,
                filters,
                requires_auth,
                permissions,
                span: start.join(end),
            });
        }

        Ok(CrudDef {
            name,
            table,
            title,
            list,
            search,
            filters,
            requires_auth,
            permissions,
            span: start.join(table_span),
        })
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
            _ => self.error(format!("expected {label}")),
        }
    }
    fn page_definition(&mut self) -> Result<PageDef, ParseError> {
        let start = self.expect(TokenKind::Page, "`page`")?;
        let path = self.string_value("page path")?;
        self.expect(TokenKind::LBrace, "`{` after page path")?;
        let mut html = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
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
            } else if self.at(&TokenKind::Requires) {
                self.advance();
                self.expect(TokenKind::Auth, "auth after requires")?;
                requires_auth = true;
            } else if self.at(&TokenKind::Permits) {
                self.advance();
                permissions.push(self.string_value("permission")?);
            } else {
                return self.error("expected `html` in page definition");
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
            requires_auth,
            permissions,
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
        let mut fields = Vec::new();
        let mut actions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Field) {
                fields.push(self.form_field()?);
            } else if self.at(&TokenKind::Fields) {
                self.advance();
                self.expect(TokenKind::LBrace, "`{` after `fields`")?;
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                    let (field_name, field_span) = self.ident("form field name")?;
                    fields.push(FormField {
                        name: field_name,
                        ty: None,
                        label: None,
                        placeholder: None,
                        required: false,
                        max: None,
                        widget: None,
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
                return self.error("expected `field`, `fields`, or `action` in form definition");
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after form definition")?;
        Ok(FormDef {
            name,
            table,
            fields,
            actions,
            span: start.join(end),
        })
    }
    fn form_field(&mut self) -> Result<FormField, ParseError> {
        let start = self.expect(TokenKind::Field, "`field`")?;
        let (name, _) = self.ident("form field name")?;
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
            max: None,
            widget: None,
            readonly: false,
            span: start,
        };
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
                TokenKind::Widget => {
                    self.advance();
                    self.expect(TokenKind::Colon, "`:` after widget")?;
                    field.widget = Some(self.form_value("widget")?);
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
        let mut success = None;
        let mut redirect = None;
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Success) {
                self.advance();
                success = Some(self.string_value("success message")?);
            } else if self.at(&TokenKind::Redirect) {
                self.advance();
                redirect = Some(self.string_value("redirect path")?);
            } else {
                statements.push(self.statement()?);
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after form action")?;
        Ok(FormAction {
            name,
            statements,
            success,
            redirect,
            span: start.join(end),
        })
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
            TokenKind::Ident(value) => {
                self.advance();
                Ok(value)
            }
            _ => self.error(format!("expected {label}")),
        }
    }
    fn table_definition(&mut self) -> Result<TableDef, ParseError> {
        let start = self.expect(TokenKind::Table, "`table`")?;
        let (name, _) = self.ident("table name")?;
        self.expect(TokenKind::LBrace, "`{` after table name")?;
        let mut columns = Vec::new();
        let mut indexes = Vec::new();
        let mut uniques = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Index) {
                indexes.push(self.index_definition(TokenKind::Index)?);
            } else if self.at(&TokenKind::Unique) {
                uniques.push(self.index_definition(TokenKind::Unique)?);
            } else {
                columns.push(self.column_definition()?);
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after table definition")?;
        Ok(TableDef {
            name,
            columns,
            indexes,
            uniques,
            span: start.join(end),
        })
    }
    fn column_definition(&mut self) -> Result<ColumnDef, ParseError> {
        let (name, span) = self.ident("column name")?;
        self.expect(TokenKind::Colon, "`:` after column name")?;
        let (ty, length) = self.column_type()?;
        let mut required = false;
        let mut primary_key = false;
        let mut auto = false;
        let mut unique = false;
        let mut default = None;
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
                    unique = true;
                    self.advance();
                }
                TokenKind::Default => {
                    self.advance();
                    default = Some(self.default_value()?);
                }
                _ => break,
            }
        }
        Ok(ColumnDef {
            name,
            ty,
            length,
            required,
            primary_key,
            auto,
            unique,
            default,
            span,
        })
    }
    fn column_type(&mut self) -> Result<(Type, Option<u32>), ParseError> {
        let ty = self.type_name()?;
        if self.at(&TokenKind::LParen) {
            if ty != Type::String {
                return self.error("length is only supported for String columns");
            }
            self.advance();
            let length = match self.current().kind.clone() {
                TokenKind::Int(value) if value > 0 => {
                    self.advance();
                    value as u32
                }
                _ => return self.error("expected positive String length"),
            };
            self.expect(TokenKind::RParen, "`)` after String length")?;
            Ok((ty, Some(length)))
        } else {
            Ok((ty, None))
        }
    }
    fn default_value(&mut self) -> Result<DefaultValue, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Int(value) => Ok(DefaultValue::Int(value)),
            TokenKind::True => Ok(DefaultValue::Bool(true)),
            TokenKind::False => Ok(DefaultValue::Bool(false)),
            TokenKind::String(value) => Ok(DefaultValue::String(value)),
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
            let (column, _) = self.ident("indexed column name")?;
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
    fn type_definition(&mut self) -> Result<TypeDef, ParseError> {
        let start = self.expect(TokenKind::Type, "`type`")?;
        let (name, _) = self.ident("type name")?;
        self.expect(TokenKind::Equal, "`=` in type definition")?;
        let target = self.type_name()?;
        Ok(TypeDef {
            name,
            target,
            span: start,
        })
    }

    fn record_definition(&mut self) -> Result<RecordDef, ParseError> {
        let start = self.expect(TokenKind::Struct, "`struct`")?;
        let (name, _) = self.ident("record name")?;
        self.expect(TokenKind::LBrace, "`{` after record name")?;
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (field_name, span) = self.ident("record field name")?;
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
            fields,
            span: start.join(end),
        })
    }
    fn function(&mut self) -> Result<Function, ParseError> {
        let start = self.expect(TokenKind::Fn, "`fn`")?;
        let (name, _) = self.ident("function name")?;
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
                capabilities.push(self.ident("capability name")?.0);
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
        let (name, _) = self.ident("type name")?;
        let mut ty = match name.as_str() {
            "Int" => Type::Int,
            "UInt" => Type::UInt,
            "Float" => Type::Float,
            "Decimal" => Type::Decimal,
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Char" => Type::Char,
            "Bytes" => Type::Bytes,
            "Timestamp" => Type::Timestamp,
            "Date" => Type::Date,
            "Time" => Type::Time,
            "Duration" => Type::Duration,
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
            _ => Type::Named(name),
        };
        if self.at(&TokenKind::Question) {
            self.advance();
            ty = Type::Option(Box::new(ty));
        }
        if self.at(&TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket, "`]` after array type")?;
            ty = Type::Array(Box::new(ty));
        }
        Ok(ty)
    }
    fn block(&mut self) -> Result<Block, ParseError> {
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
            let start = self.advance().span;
            let condition = self.expression()?;
            let then_block = self.block()?;
            self.skip_newlines();
            let else_block = if self.at(&TokenKind::Else) {
                self.advance();
                Some(self.block()?)
            } else {
                None
            };
            let end = else_block.as_ref().map_or(then_block.span, |b| b.span);
            return Ok(Stmt::If {
                condition,
                then_block,
                else_block,
                span: start.join(end),
            });
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
            let (name, _) = self.ident("loop variable name")?;
            self.expect(TokenKind::In, "`in` after loop variable")?;
            let iterable = self.expression()?;
            let body = self.block()?;
            return Ok(Stmt::For {
                name,
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
                self.expect(TokenKind::FatArrow, "`=>` after match pattern")?;
                let body = self.block()?;
                let span = pattern.span.join(body.span);
                arms.push(MatchArm {
                    pattern,
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
        Ok(Stmt::Expr(self.expression()?))
    }
    fn pattern(&mut self) -> Result<Pattern, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) if name == "_" => Ok(Pattern {
                kind: PatternKind::Wildcard,
                span: token.span,
            }),
            TokenKind::Ident(name) => {
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
            TokenKind::Char(value) => Ok(Pattern {
                kind: PatternKind::Char(value),
                span: token.span,
            }),
            found => self.error(format!("expected pattern, found {found:?}")),
        }
    }
    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.binary(0)
    }
    fn binary(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.unary()?;
        loop {
            self.skip_newlines();
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
        self.skip_newlines();
        if self.at(&TokenKind::Minus) {
            let span = self.advance().span;
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
                let (field, end) = self.ident("field name after `.`")?;
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
            TokenKind::Int(value) => Ok(Expr {
                kind: ExprKind::Int(value),
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
            TokenKind::Ident(name) => {
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
                        kind: ExprKind::Call { name, args },
                        span: token.span.join(end),
                    })
                } else if self.at(&TokenKind::LBrace) && self.looks_like_record_literal() {
                    self.record_literal(name, token.span)
                } else {
                    Ok(Expr {
                        kind: ExprKind::Variable(name),
                        span: token.span,
                    })
                }
            }
            TokenKind::LParen => {
                let expr = self.expression()?;
                self.expect(TokenKind::RParen, "`)`")?;
                Ok(expr)
            }
            found => self.error(format!("expected expression, found {found:?}")),
        }
    }

    fn record_literal(&mut self, type_name: String, start: Span) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace, "`{` after record type")?;
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let (field, _) = self.ident("record field name")?;
            self.expect(TokenKind::Colon, "`:` after record field name")?;
            let value = self.expression()?;
            fields.push((field, value));
            self.skip_newlines();
            if self.at(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }
        let end = self.expect(TokenKind::RBrace, "`}` after record literal")?;
        Ok(Expr {
            kind: ExprKind::Record { type_name, fields },
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
            (Some(TokenKind::Ident(_)), Some(TokenKind::Colon))
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
    fn parses_explicit_and_schema_mapped_form() {
        let source = r#"
            form CustomerForm {
                field email: Email {
                    label: "E-Mail"
                    required
                    max: 255
                    widget: email
                }
                action save {
                    sql {
                        INSERT INTO customers (email) VALUES (:email)
                    }
                    success "Saved"
                    redirect "/customers"
                }
            }
            form CustomerCreate -> customers {
                fields {
                    name
                    active
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert_eq!(program.forms.len(), 2);
        assert_eq!(program.forms[0].fields[0].widget.as_deref(), Some("email"));
        assert_eq!(
            program.forms[0].actions[0].redirect.as_deref(),
            Some("/customers")
        );
        assert!(matches!(
            program.forms[0].actions[0].statements[0],
            Stmt::Expr(Expr {
                kind: ExprKind::Sql { .. },
                ..
            })
        ));
        assert_eq!(program.forms[1].table.as_deref(), Some("customers"));
        assert_eq!(program.forms[1].fields.len(), 2);
    }

    #[test]
    fn parses_minimal_crud_definition() {
        let program = parse(&lex("crud Machine -> machines").unwrap()).unwrap();
        assert_eq!(program.cruds.len(), 1);
        assert_eq!(program.cruds[0].name, "Machine");
        assert_eq!(program.cruds[0].table, "machines");
    }

    #[test]
    fn parses_configured_crud_definition() {
        let program = parse(
            &lex(r#"crud Customer -> customers {
                    title: "Customers"
                    list { customer_number name }
                    search { name }
                    filter { active }
                }"#)
            .unwrap(),
        )
        .unwrap();
        let crud = &program.cruds[0];
        assert_eq!(crud.title.as_deref(), Some("Customers"));
        assert_eq!(crud.list, ["customer_number", "name"]);
        assert_eq!(crud.search, ["name"]);
        assert_eq!(crud.filters, ["active"]);
    }

    #[test]
    fn parses_auth_and_protected_crud_definition() {
        let program = parse(
            &lex(r#"auth users {
                    table: users
                    sessions: auth_sessions
                    permissions: user_permissions
                }

                crud Customer -> customers {
                    requires auth
                    permits "customers.view"
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
        assert!(program.cruds[0].requires_auth);
        assert_eq!(program.cruds[0].permissions, ["customers.view"]);
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
    fn parses_array_for_loops() {
        let program =
            parse(&lex("fn main() { for value in [1, 2] { print(value) } }").unwrap()).unwrap();
        assert!(matches!(
            program.functions[0].body.statements[0],
            Stmt::For { ref name, .. } if name == "value"
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
}
