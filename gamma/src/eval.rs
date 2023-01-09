use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};
use gamma_parser::{ast, parser::Parser};
use std::io::Write;
use std::{collections::HashMap, process::exit};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
type Context = HashMap<String, (ast::Span, ast::Expression, ast::Span)>;

pub struct Evaluator<'a> {
    file_span: codemap::Span,
    context: Context,
    ast: ast::AST,
    emitter: Emitter<'a>,
}

impl<'a> Evaluator<'a> {
    pub fn new(source: &'a str, filename: &'a str, codemap: &'a mut CodeMap) -> Self {
        let context = HashMap::new();
        let mut parser = Parser::new(source, filename, codemap);
        let ast = parser.parse();

        let file_span = parser.file_span;
        let emitter = Emitter::stderr(ColorConfig::Always, Some(parser.codemap));

        Self {
            file_span,
            context,
            ast,
            emitter,
        }
    }

    pub fn eval(&mut self) -> bool {
        for statement in self.ast.to_owned() {
            if !self.eval_statement(statement) {
                let mut stdout = StandardStream::stdout(ColorChoice::Always);
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
                writeln!(
                    &mut stdout,
                    "error: aborting due to error occured in execution process"
                );
                stdout.set_color(ColorSpec::new().set_fg(None));
                return false;
            }
        }

        true
    }

    fn eval_statement(&mut self, statement: ast::Statement) -> bool {
        match statement {
            ast::Statement::Let {
                name,
                name_span,
                expression,
                expression_span,
                span: _,
            } => {
                if self.context.contains_key(&name) {
                    self.emitter.emit(&[
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
                        Diagnostic {
                            level: Level::Note,
                            message: "consider renaming the variable".to_owned(),
                            spans: vec![SpanLabel {
                                span: self.logos_to_codemap_span(&name_span),
                                style: SpanStyle::Primary,
                                label: Some(format!("rename `{}` here", name).to_owned()),
                            }],
                            code: Some("N003".to_owned()),
                        },
                    ]);

                    return false;
                }

                self.context
                    .insert(name, (name_span, expression, expression_span));
            }
            _ => {}
        }

        true
    }

    fn logos_to_codemap_span(&self, logos_span: &ast::Span) -> codemap::Span {
        return self
            .file_span
            .subspan(logos_span.start as u64, logos_span.end as u64);
    }
}
