use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum RawToken {
    #[regex(r"[ \t\n\r\f]+", logos::skip)]
    #[token(";")]
    Semicolon,

    #[token("$")]
    Dollar,

    #[token("=>")]
    RightArrow,

    #[token(".")]
    Period,

    #[token("=")]
    Assign,

    #[token("lambda")]
    #[token(r#"\"#)]
    #[token("λ")]
    Lambda,

    #[token("let")]
    Let,

    #[token("(")]
    Lparen,

    #[token(")")]
    Rparen,

    #[regex(r"[_0-9a-zA-Z]+")]
    Identifier,

    #[error]
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub raw: RawToken,
    pub span: std::ops::Range<usize>,
    pub literal: String,
}

pub fn lex(src: &str) -> impl Iterator<Item = Token> + '_ {
    RawToken::lexer(src)
        .spanned()
        .map(|(raw, span)| Token {
            raw: raw,
            span: span.clone(),
            literal: src[span.start..span.end].to_owned(),
        })
        .peekable()
}
