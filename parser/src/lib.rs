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
    fn program(mut self) -> Result<Program, ParseError> {
        let mut databases = Vec::new();
        let mut tables = Vec::new();
        let mut types = Vec::new();
        let mut records = Vec::new();
        let mut views = Vec::new();
        let mut components = Vec::new();
        let mut pages = Vec::new();
        let mut tableviews = Vec::new();
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
            } else if self.at(&TokenKind::View) {
                views.push(self.view_definition()?);
            } else if self.at(&TokenKind::Component) {
                components.push(self.component_definition()?);
            } else if self.at(&TokenKind::Page) {
                pages.push(self.page_definition()?);
            } else if self.at(&TokenKind::TableView) {
                tableviews.push(self.tableview_definition()?);
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
            views,
            components,
            pages,
            tableviews,
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
        let mut roles_table = None;
        let mut role_permissions_table = None;
        let mut audit_table = None;
        let mut audit_chain = false;
        let mut admin_path = None;
        let mut admin_permission = None;
        let mut admin_role = None;
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
                _ => return self.error("expected authentication option"),
            };
            self.advance();
            self.expect(TokenKind::Colon, "colon after authentication option")?;
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
            let value = if matches!(field, "admin_path" | "admin_permission") {
                self.string_value("authentication option value")?
            } else {
                self.ident("authentication option value")?.0
            };
            match field {
                "table" => table = Some(value),
                "sessions" => session_table = Some(value),
                "permissions" => permissions_table = Some(value),
                "roles" => roles_table = Some(value),
                "role_permissions" => role_permissions_table = Some(value),
                "audit" => audit_table = Some(value),
                "admin_path" => admin_path = Some(value),
                "admin_permission" => admin_permission = Some(value),
                "admin_role" => admin_role = Some(value),
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
            roles_table,
            role_permissions_table,
            audit_table,
            audit_chain,
            admin_path,
            admin_permission,
            admin_role,
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
        let mut view = CrudViewDef::default();
        let mut soft_delete = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        let mut create_permissions = Vec::new();
        let mut edit_permissions = Vec::new();
        let mut delete_permissions = Vec::new();
        let mut actions = Vec::new();

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
                        match scope.as_deref() {
                            Some("create") => create_permissions.push(permission),
                            Some("edit") => edit_permissions.push(permission),
                            Some("delete") => delete_permissions.push(permission),
                            Some("view") | None => permissions.push(permission),
                            _ => unreachable!("CRUD permission scope was validated above"),
                        }
                    }
                    TokenKind::Action => {
                        actions.push(self.form_action()?);
                    }
                    _ => {
                        return self.error(
                            "expected title, list, search, filter, view, requires auth, permits, or action in CRUD definition",
                        )
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
                view,
                soft_delete,
                requires_auth,
                permissions,
                create_permissions,
                edit_permissions,
                delete_permissions,
                actions,
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
            view,
            soft_delete,
            requires_auth,
            permissions,
            create_permissions,
            edit_permissions,
            delete_permissions,
            actions,
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
                        "expected `fields`, `list`, `detail`, `form`, `delete`, `loading`, or `error` in CRUD view definition",
                    )
                }
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace, "`}` after CRUD view definition")?;
        Ok(view)
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
                return self.error("expected `load`, `html`, `view`, `paginated`, `requires auth`, or `permits` in page definition");
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

    fn view_definition(&mut self) -> Result<ViewDef, ParseError> {
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
            html,
            span: start.join(end),
        })
    }

    fn tableview_definition(&mut self) -> Result<TableViewDef, ParseError> {
        let start = self.expect(TokenKind::TableView, "`tableview`")?;
        let (name, _) = self.ident("tableview name")?;
        self.expect(TokenKind::LBrace, "`{` after tableview name")?;
        let mut result_type = None;
        let mut source = None;
        let mut columns = Vec::new();
        let mut filters = Vec::new();
        let mut searchable = false;
        let mut sortable = false;
        let mut page_size = None;
        let mut requires_auth = false;
        let mut permissions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Source) {
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
                    "expected source, columns, filter, searchable, sortable, paginated, requires auth, or permits in tableview definition",
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
            result_type,
            source,
            columns,
            filters,
            searchable,
            sortable,
            page_size,
            requires_auth,
            permissions,
            span: start.join(end),
        })
    }

    fn component_definition(&mut self) -> Result<ComponentDef, ParseError> {
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
            } else {
                statements.push(self.statement()?);
            }
            self.skip_newlines();
        }
        let end = self.expect(TokenKind::RBrace, "`}` after form action")?;
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
            statements,
            success,
            redirect,
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
            "Map" => {
                self.expect(TokenKind::Less, "`<` after `Map`")?;
                let key = self.type_name()?;
                self.expect(TokenKind::Comma, "`,` between Map key and value types")?;
                let value = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after Map value type")?;
                Type::Map(Box::new(key), Box::new(value))
            }
            "HttpResult" => {
                self.expect(TokenKind::Less, "`<` after `HttpResult`")?;
                let response = self.type_name()?;
                self.expect(TokenKind::Greater, "`>` after HttpResult type")?;
                Type::HttpResult(Box::new(response))
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
                let type_args =
                    if matches!(name.as_str(), "json_decode" | "http_json" | "http_result")
                        && self.at(&TokenKind::Less)
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
    fn parses_array_for_loops() {
        let program =
            parse(&lex("fn main() { for value in [1, 2] { print(value) } }").unwrap()).unwrap();
        assert!(matches!(
            program.functions[0].body.statements[0],
            Stmt::For { ref name, .. } if name == "value"
        ));
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
}
