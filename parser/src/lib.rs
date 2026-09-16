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
        let mut functions = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::Eof) {
            functions.push(self.function()?);
            self.skip_newlines();
        }
        Ok(Program { functions })
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
        Ok(match name.as_str() {
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
            _ => Type::Named(name),
        })
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
