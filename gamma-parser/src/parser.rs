use crate::lexer::{RawToken, Token};
use crate::{
    ast::{self, *},
    lexer,
};
use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};

macro_rules! true_or_return_none {
    ($a: expr) => {
        if (!$a) {
            return None;
        }
    };
}

macro_rules! if_none_return_none {
    ($a: expr) => {
        if ($a.is_none()) {
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

pub struct Parser<'a> {
    source: &'a str,
    filename: &'a str,
    previous_token_span: Option<ast::Span>,
    token: Option<Token>,
    tokens_iterator: Box<dyn Iterator<Item = Token> + 'a>,
    file_span: codemap::Span,
    codemap: CodeMap,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, filename: &'a str) -> Parser<'a> {
        let mut codemap = CodeMap::new();
        let file_span = codemap
            .add_file(filename.to_owned(), source.to_owned())
            .span;

        let mut parser = Self {
            source: source,
            filename: filename,
            codemap: codemap,
            previous_token_span: None,
            token: None,
            file_span: file_span,
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

        true_or_return_none!(self.check_token(
            RawToken::Identifier,
            "expected name of variable in let statement".to_owned(),
        ));

        let name = self.token.as_ref().unwrap().clone().literal;
        let name_span = self.token.as_ref().unwrap().clone().span;

        self.consume_token();

        true_or_return_none!(self.check_token(
            RawToken::Assign,
            "help: consider adding '=' in the let statement".to_owned(),
        ));

        self.consume_token();

        let expression_result = self.parse_expression();

        if_none_return_none!(expression_result);

        let (expression, expression_span) = expression_result.unwrap();

        check_eof!(self);

        let end = self.token.as_ref().unwrap().span.end;

        true_or_return_none!(self.check_token(
            RawToken::Semicolon,
            "help: consider adding ';' at the end of the let statement".to_owned(),
        ));

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
        let expression_result = self.parse_expression();

        if_none_return_none!(expression_result);

        let (expression, expression_span) = expression_result.unwrap();

        let start = expression_span.start;

        true_or_return_none!(self.check_token(
            RawToken::Semicolon,
            "help: consider adding ';' at the end of expression statement".to_owned(),
        ));

        let end = self.token.as_ref().unwrap().span.end;

        Some(Statement::Expression {
            expression: expression,
            expression_span: expression_span,
            span: start..end,
        })
    }

    fn parse_expression(&mut self) -> Option<(Expression, ast::Span)> {
        check_eof!(self);

        match self.token.as_ref().unwrap().raw {
            RawToken::Identifier => {
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
            RawToken::Lparen => {
                let start = self.token.as_ref().unwrap().clone().span.start;
                self.consume_token();

                let expression_result = self.parse_expression();

                if_none_return_none!(expression_result);

                let (expression, expression_span) = expression_result.unwrap();

                true_or_return_none!(self.check_token(
                    RawToken::Rparen,
                    "help: consider adding ')' at the end of parenthesised expression".to_owned(),
                ));

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
            RawToken::Lambda => {
                let start = self.token.as_ref().unwrap().clone().span.start;
                self.consume_token();

                true_or_return_none!(
                    self.check_token(RawToken::Identifier, "expected argument name".to_owned())
                );

                let name = self.token.as_ref().unwrap().clone().literal;
                let name_span = self.token.as_ref().unwrap().clone().span;

                self.consume_token();

                true_or_return_none!(self.check_token(
                    RawToken::RightArrow,
                    "help: consider adding '->'".to_owned(),
                ));

                self.consume_token();

                let expression_result = self.parse_expression();

                if_none_return_none!(expression_result);

                let (expression, expression_span) = expression_result.unwrap();

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
        Emitter::stderr(ColorConfig::Always, Some(&self.codemap)).emit(&[Diagnostic {
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
        Emitter::stderr(ColorConfig::Always, Some(&self.codemap)).emit(&[Diagnostic {
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
        if self.token.is_none() {
            self.unexpected_eof();
            return false;
        }

        if self.token.as_ref().unwrap().raw != expected {
            Emitter::stderr(ColorConfig::Always, Some(&self.codemap)).emit(&[Diagnostic {
                level: Level::Error,
                message: "parsing error found".to_owned(),
                spans: vec![SpanLabel {
                    span: self.token_span(&self.token),
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
    use crate::parser::Expression::*;
    use crate::parser::Statement::*;

    #[test]
    fn let_id() {
        assert_eq!(
            Parser::new("let a = x;", "<stdin>").parse()[0],
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
            Parser::new("let a = \\x -> x;", "<stdin>").parse()[0],
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
