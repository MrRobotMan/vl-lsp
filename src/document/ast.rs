use crate::document::{Span, lexer::Token};

pub fn parse(_tokens: &[Token], _source: &str) -> Ast {
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    root: Node,
    errors: Vec<ParseError>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Node {
    span: Span,
    kind: NodeKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum NodeKind {
    List,
    Symbol,
    Integer,
    Real,
    True,
    Nil,
    Quote,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ParseError {
    span: Span,
    kind: ParseErrorKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ParseErrorKind {
    MisplacedDot,
    QuoteMissingOperand,
    UnclosedList,
    UnmatchedCloseParen,
    UnterminatedString,
    UnterminatedBlockComment,
}

#[cfg(test)]
mod ast_tests;
