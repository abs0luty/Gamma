pub type Span = std::ops::Range<usize>;

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Var {
        name: String,
        name_span: Span,
    },
    Apply {
        lhs: Box<Expression>,
        lhs_span: Span,
        rhs: Box<Expression>,
        rhs_span: Span,
    },
    Paren {
        expression: Box<Expression>,
        expression_span: Span,
    },
    Abstraction {
        name: String,
        name_span: Span,
        expression: Box<Expression>,
        expression_span: Span,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    Expression {
        expression: Expression,
        expression_span: Span,
        span: Span,
    },
    Let {
        name: String,
        name_span: Span,
        expression: Expression,
        expression_span: Span,
        span: Span,
    },
}

pub type AST = Vec<Statement>;
