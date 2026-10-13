use super::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token<'t> {
    span: Span,
    kind: TokenKind<'t>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind<'t> {
    OpenParen,
    CloseParen,
    Dot,
    Quote,
    Comment(&'t str),
    BlockComment(&'t str),
    String(&'t str),
    Real(f64),
    Integer(i32),
    Symbol(&'t str),
    Error(ErrorKind),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnterminatedString,
    UnterminatedBlockComment,
}

pub fn tokenize<'t>(_source: &'t str) -> Vec<Token<'t>> {
    Vec::new()
}
