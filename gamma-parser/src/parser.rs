use std::borrow::Borrow;

use crate::ast::{self, *};
use crate::lexer::{RawToken, Token};
use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};

macro_rules! check_token {
    ($self: expr, $rawtoken: expr, $msg: expr) => {
        if !$self.check_token($rawtoken, $msg) {
            return None;
        }
    };
}

macro_rules! check_eof {
    ($self: expr) => {
        if $self.token.is_none() {
            $self.unexpected_eof();
            return None;
        }
    };
}

/// Grammar for Gamma:
///
/// Program   ::= Statement* EOF
/// Statement ::= (Let | Expression) ";"
/// Let       ::= "let" Identifier "=" Expression
/// Expression ::= Abstraction
///                | Application
///                | Identifier
///                | "(" Expression ")"
/// Application ::= Function Argument
/// Function ::= Identifier
///            | Application
///            | "(" Expression ")"
/// Argument ::= Identifier
///            | "(" Expression ")"
pub struct Parser<'a> {
    pub codemap: &'a CodeMap,
    pub file_span: codemap::Span,
    pub emitter: Emitter<'a>,
    previous_token_span: Option<ast::Span>,
    token: Option<Token>,
    tokens_iterator: Box<dyn Iterator<Item = Token> + 'a>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, filename: &'a str, codemap: &'a mut CodeMap) -> Parser<'a> {
        let file_span = codemap
            .add_file(filename.to_owned(), source.to_owned())
            .span;

        let codemap_imut: &_ = codemap;

        let mut parser = Self {
            codemap: codemap_imut,
            previous_token_span: None,
            token: None,
            file_span: file_span,
            emitter: Emitter::stderr(ColorConfig::Always, Some(codemap_imut)),
            tokens_iterator: Box::new(crate::lexer::lex(source)),
        };

        parser.consume_token();
        parser
    }

    pub fn parse(&mut self) -> AST {
        let mut ast = vec![];

        while self.token.is_some() {
            let statement = self.parse_statement();
            if statement.is_some() {
                ast.push(statement.unwrap());
            }
        }

        ast
    }

    pub fn parse_statement(&mut self) -> Option<Statement> {
        check_eof!(self);

        match self.token.as_ref().unwrap().raw {
            RawToken::Let => self.parse_let_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Option<Statement> {
        let start = self.token.as_ref().unwrap().span.start;

        self.consume_token();

        check_token!(
            self,
            RawToken::Identifier,
            "expected name of variable in the let statement".to_owned()
        );

        let name = self.token.as_ref().unwrap().clone().literal;
        let name_span = self.token.as_ref().unwrap().clone().span;

        self.consume_token();

        check_token!(
            self,
            RawToken::Assign,
            "help: consider adding '=' in the let statement".to_owned()
        );

        self.consume_token();

        let (expression, expression_span) = self.parse_expression()?;

        check_token!(
            self,
            RawToken::Semicolon,
            "help: consider adding ';' at the end of the let statement".to_owned()
        );

        let end = self.token.as_ref().unwrap().span.end;

        self.consume_token();

        Some(Statement::Let {
            name: name,
            name_span: name_span,
            expression: expression,
            expression_span: expression_span,
            span: start..end,
        })
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let (expression, expression_span) = self.parse_expression()?;

        let start = expression_span.start;

        check_token!(
            self,
            RawToken::Semicolon,
            "help: consider adding ';' at the end of expression statement".to_owned()
        );

        let end = self.token.as_ref().unwrap().span.end;

        self.consume_token();

        Some(Statement::Expression {
            expression: expression,
            expression_span: expression_span,
            span: start..end,
        })
    }

    fn parse_name_expression(&mut self) -> Option<(Expression, ast::Span)> {
        let name = self.token.as_ref().unwrap().clone().literal;
        let name_span = self.token.as_ref().unwrap().clone().span;

        self.consume_token();

        Some((
            ast::Expression::Var {
                name: name,
                name_span: name_span.clone(),
            },
            name_span,
        ))
    }

    fn parse_paren_expression(&mut self) -> Option<(Expression, ast::Span)> {
        let start = self.token.as_ref().unwrap().clone().span.start;
        self.consume_token();

        let (expression, expression_span) = self.parse_expression()?;

        check_token!(
            self,
            RawToken::Rparen,
            "help: consider adding ')' at the end of parenthesised expression".to_owned()
        );

        self.consume_token();

        check_eof!(self);

        let end = self.token.as_ref()?.clone().span.start;
        Some((
            Expression::Paren {
                expression: Box::new(expression),
                expression_span: expression_span,
            },
            start..end,
        ))
    }

    fn parse_abstraction_expression(&mut self) -> Option<(Expression, ast::Span)> {
        let start = self.token.as_ref().unwrap().clone().span.start;
        self.consume_token();

        check_token!(
            self,
            RawToken::Identifier,
            "expected argument name".to_owned()
        );

        let name = self.token.as_ref().unwrap().clone().literal;
        let name_span = self.token.as_ref().unwrap().clone().span;

        self.consume_token();

        if self.token.is_some() && self.token.as_ref().unwrap().raw == RawToken::Period {
            self.emitter.emit(&[Diagnostic {
                        level: Level::Warning,
                        message: "use '=>' in abstractions".to_owned(),
                        spans: vec![SpanLabel {
                            span: self.token_span(&self.token),
                            style: SpanStyle::Primary,
                            label: Some("help: use '=>' instead of '.' because Gamma uses different syntax rather than usual one in Lambda calculus.".to_owned()),
                        }],
                        code: Some("W002".to_owned()),
                    }]);
        } else {
            check_token!(
                self,
                RawToken::RightArrow,
                "help: consider adding '=>'".to_owned()
            );
        }

        self.consume_token();

        let (expression, expression_span) = self.parse_expression()?;

        check_eof!(self);

        let end = self.token.as_ref()?.clone().span.start;

        Some((
            Expression::Abstraction {
                name: name,
                name_span: name_span,
                expression: Box::new(expression),
                expression_span: expression_span,
            },
            start..end,
        ))
    }

    fn parse_application(&mut self) -> Option<(Expression, ast::Span)> {
        let mut expressions = vec![];
        loop {
            check_eof!(self);

            match self.token.as_ref().unwrap().raw {
                RawToken::Identifier => {
                    expressions.push(self.parse_name_expression()?);
                }
                RawToken::Lparen => {
                    expressions.push(self.parse_paren_expression()?);
                }
                _ => {
                    return match expressions.len() {
                        0 => {
                            self.unexpected_token("do not write empty expressions".to_owned());
                            None
                        }
                        1 => Some(expressions[0].to_owned()),
                        _ => {
                            let mut accumulator = expressions.pop()?;
                            let start_defined = false;
                            let mut start = 0;
                            while !expressions.is_empty() {
                                let (rhs, rhs_span) = accumulator;
                                let (lhs, lhs_span) = expressions.pop().unwrap();

                                if !start_defined {
                                    start = lhs_span.start;
                                }

                                let end = rhs_span.end;
                                accumulator = (
                                    Expression::Apply {
                                        lhs: Box::new(lhs),
                                        lhs_span,
                                        rhs: Box::new(rhs),
                                        rhs_span,
                                    },
                                    start..end,
                                )
                            }

                            Some(accumulator)
                        }
                    };
                }
            }
        }
    }

    fn parse_expression(&mut self) -> Option<(Expression, ast::Span)> {
        check_eof!(self);

        match self.token.as_ref().unwrap().raw {
            RawToken::Lambda => self.parse_abstraction_expression(),
            RawToken::Lparen => self.parse_application(),
            RawToken::Identifier => self.parse_application(),
            _ => {
                self.unexpected_token(
                    "expression must start with identifier, 'lambda', '\\' or '('".to_owned(),
                );
                self.consume_token();
                None
            }
        }
    }

    fn consume_token(&mut self) {
        let previous_token = self.token.as_ref();
        if previous_token.is_some() {
            self.previous_token_span = Some(previous_token.unwrap().span.clone());
        } else {
            self.previous_token_span = None;
        }

        self.token = self.tokens_iterator.next();
    }

    fn token_span(&self, token: &Option<Token>) -> codemap::Span {
        return self.file_span.subspan(
            token.as_ref().unwrap().span.start as u64,
            token.as_ref().unwrap().span.end as u64,
        );
    }

    fn span(&self, span: &ast::Span) -> codemap::Span {
        return self.file_span.subspan(span.start as u64, span.end as u64);
    }

    pub fn unexpected_eof(&mut self) {
        self.emitter.emit(&[Diagnostic {
            level: Level::Error,
            spans: vec![SpanLabel {
                span: match self.previous_token_span.as_ref() {
                    Some(_) => self.span(self.previous_token_span.as_ref().unwrap()),
                    None => self.file_span.subspan(0, 1),
                },
                style: SpanStyle::Primary,
                label: Some("unexpected end of file/input".to_owned()),
            }],
            message: "parsing error found".to_owned(),
            code: Some("E001".to_owned()),
        }]);
    }

    pub fn unexpected_token(&mut self, message: String) {
        self.emitter.emit(&[Diagnostic {
            level: Level::Error,
            spans: vec![SpanLabel {
                span: self.token_span(&self.token),
                style: SpanStyle::Primary,
                label: Some(message),
            }],
            message: "parsing error found".to_owned(),
            code: Some("E001".to_owned()),
        }]);
    }

    pub fn check_token(&mut self, expected: RawToken, message: String) -> bool {
        if self.token.is_none() || self.token.as_ref().unwrap().raw != expected {
            self.emitter.emit(&[Diagnostic {
                level: Level::Error,
                message: "parsing error found".to_owned(),
                spans: vec![SpanLabel {
                    span: if self.token.is_none() {
                        match self.previous_token_span.as_ref() {
                            Some(_) => self.span(self.previous_token_span.as_ref().unwrap()),
                            None => self.file_span.subspan(0, 1),
                        }
                    } else {
                        self.token_span(&self.token)
                    },
                    style: SpanStyle::Primary,
                    label: Some(message),
                }],
                code: Some("E001".to_owned()),
            }]);

            self.consume_token();

            return false;
        }

        true
    }
}

#[cfg(test)]
mod parser_tests {
    use super::Parser;
    use crate::parser::CodeMap;
    use crate::parser::Expression::*;
    use crate::parser::Statement::*;

    #[test]
    fn let_id() {
        assert_eq!(
            Parser::new("let a = x;", "<stdin>", &mut CodeMap::new()).parse()[0],
            Let {
                name: "a".to_string(),
                name_span: 4..5,
                expression: Var {
                    name: "x".to_string(),
                    name_span: 8..9
                },
                expression_span: 8..9,
                span: 0..10
            }
        );
    }

    #[test]
    fn let_lambda() {
        assert_eq!(
            Parser::new("let a = \\x => x;", "<stdin>", &mut CodeMap::new()).parse()[0],
            Let {
                name: "a".to_string(),
                name_span: 4..5,
                expression: Abstraction {
                    name: "x".to_string(),
                    name_span: 9..10,
                    expression: Box::new(Var {
                        name: "x".to_string(),
                        name_span: 14..15
                    }),
                    expression_span: 14..15
                },
                expression_span: 8..15,
                span: 0..16
            }
        );
    }
}
