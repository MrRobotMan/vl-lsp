use std::collections::VecDeque;

use crate::span::Span;

#[derive(Debug, Clone)]
enum Token {
    OpenParen(Span),
    CloseParen(Span),
    Dot(Span),
    Quote(Span),
    DoubleQuote(Span),
    Comment(Span),
    BlockCommentStart(Span),
    BlockCommentEnd(Span),
    String(String, Span),
    Real(f64, Span),
    Integer(i32, Span),
    Symbol(String, Span),
    Error(Span),
}

fn tokenize(source: VecDeque<char>) -> Vec<Token> {
    todo!()
}
