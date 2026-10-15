use crate::document::{Span, lexer::Token};

pub fn parse(_tokens: &[Token], source: &str) -> Ast {
    let ast = Ast::new(source);
    debug_assert!(ast.root.root_count() == 1);
    ast
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    root: Node,
    errors: Vec<ParseError>,
}

impl Ast {
    fn new(source: &str) -> Self {
        Self {
            root: Node {
                span: Span {
                    start: 0,
                    end: source.len(),
                },
                kind: NodeKind::Root(Vec::new()),
            },
            errors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    span: Span,
    kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeKind {
    Root(Vec<Node>),
    List(Vec<Node>),
    Symbol,
    Dot,
    Integer,
    Real,
    String,
    True,
    Nil,
    Quote(Box<Node>),
    Missing,
}

impl Node {
    #[cfg(debug_assertions)]
    fn root_count(&self) -> usize {
        match &self.kind {
            NodeKind::Root(v) => 1 + v.iter().map(|n| n.root_count()).sum::<usize>(),
            NodeKind::List(v) => v.iter().map(|n| n.root_count()).sum::<usize>(),
            NodeKind::Quote(n) => n.root_count(),
            _ => 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ParseError {
    span: Span,
    kind: ParseErrorKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ParseErrorKind {
    ExtraCdrsInDottedPair,
    MisplacedDot,
    QuoteMissingOperand,
    UnclosedList,
    UnmatchedCloseParen,
    UnterminatedString,
    UnterminatedBlockComment,
}

#[cfg(test)]
mod ast_tests;
