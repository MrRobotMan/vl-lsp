use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
struct Token<'t> {
    span: Span,
    kind: TokenKind<'t>,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind<'t> {
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
enum ErrorKind {
    UnterminatedString,
    UnterminatedBlockComment,
}

fn tokenize<'t>(source: &'t str) -> Vec<Token<'t>> {
    todo!()
}
