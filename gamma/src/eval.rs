use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};
use gamma_parser::{ast, parser::Parser};
use std::collections::HashMap;

type Context = HashMap<String, (ast::Span, ast::Expression, ast::Span)>;

pub struct Evaluator {
    file_span: codemap::Span,
    codemap: CodeMap,
    context: Context,
    ast: ast::AST,
}

impl Evaluator {
    pub fn new(source: &str, filename: &str) -> Self {
        let context = HashMap::new();
        let mut parser = Parser::new(source, filename);
        let ast = parser.parse();

        let file_span = parser.file_span;
        let codemap = parser.codemap;

        Self {
            file_span,
            codemap,
            context,
            ast,
        }
    }

    pub fn eval(&mut self) {
        for statement in self.ast.to_owned() {
            self.eval_statement(statement);
        }
    }

    fn eval_statement(&mut self, statement: ast::Statement) {
        match statement {
            ast::Statement::Let {
                name,
                name_span,
                expression,
                expression_span,
                span: _,
            } => {
                if self.context.contains_key(&name) {
                    Emitter::stderr(ColorConfig::Always, Some(&self.codemap)).emit(&[
                        Diagnostic {
                            level: Level::Error,
                            message: "trying to redefine existing variable".to_owned(),
                            spans: vec![
                                SpanLabel {
                                    span: self.logos_to_codemap_span(&name_span),
                                    style: SpanStyle::Primary,
                                    label: Some(
                                        format!("trying to overwrite `{}`", name).to_owned(),
                                    ),
                                },
                                SpanLabel {
                                    span: self.logos_to_codemap_span(&expression_span),
                                    style: SpanStyle::Secondary,
                                    label: Some("new value".to_owned()),
                                },
                            ],

                            code: Some("E003".to_owned()),
                        },
                        Diagnostic {
                            level: Level::Note,
                            message: format!("variable `{}` was firstly defined here", name)
                                .to_owned(),
                            spans: vec![SpanLabel {
                                span: self
                                    .logos_to_codemap_span(&self.context.get(&name).unwrap().2),
                                style: SpanStyle::Primary,
                                label: Some("previous value".to_owned()),
                            }],
                            code: Some("N003".to_owned()),
                        },
                    ]);
                }

                self.context
                    .insert(name, (name_span, expression, expression_span));
            }
            _ => {}
        }
    }

    fn logos_to_codemap_span(&self, logos_span: &ast::Span) -> codemap::Span {
        return self
            .file_span
            .subspan(logos_span.start as u64, logos_span.end as u64);
    }
}
