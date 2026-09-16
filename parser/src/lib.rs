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
        let mut functions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Database) {
                databases.push(self.database_definition()?);
            } else if self.at(&TokenKind::Table) {
                tables.push(self.table_definition()?);
            } else if self.at(&TokenKind::Type) {
                types.push(self.type_definition()?);
            } else {
                functions.push(self.function()?);
            }
            self.skip_newlines();
        }
        Ok(Program {
            databases,
            tables,
            types,
            functions,
        })
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
            engine: engine.unwrap_or_else(|| "postgres".into()),
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
        let body = self.block()?;
        Ok(Function {
            name,
            params,
            return_type,
            span: start.join(body.span),
            body,
        })
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
            let body = self.block()?;
            return Ok(Stmt::While {
                condition,
                body: body.clone(),
                span: start.join(body.span),
            });
        }
        if self.at(&TokenKind::Loop) {
            let start = self.advance().span;
            let body = self.block()?;
            return Ok(Stmt::Loop {
                body: body.clone(),
                span: start.join(body.span),
            });
        }
        if self.at(&TokenKind::Break) {
            let span = self.advance().span;
            return Ok(Stmt::Break { span });
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
        self.primary()
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
}
