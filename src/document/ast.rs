use crate::document::{
    Span,
    lexer::{ErrorKind, Token, TokenKind},
};

pub fn parse<'a>(tokens: &'a [Token], source: &'a str) -> Ast {
    let mut builder = AstBuilder::new(tokens, source);
    let ast = builder.parse();
    debug_assert!(ast.root.root_count() == 1);
    ast
}

struct AstBuilder<'a> {
    source: &'a str,
    tokens: &'a [Token],
    position: usize,
    previous_end: usize,
    errors: Vec<ParseError>,
}

impl<'a> AstBuilder<'a> {
    fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self {
            source,
            tokens,
            position: 0,
            previous_end: 0,
            errors: Vec::new(),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.position];
        self.position += 1;
        self.previous_end = t.span.end;
        t
    }

    /// Recovery points are `defun` and `defun-*` at line starts.
    /// Check if the current position is an open paren at the start of a line or
    /// following a CR or LF character and the next token is a defun symbol.
    fn at_recover_point(&self) -> bool {
        if let Some(Token {
            span: cur_span,
            kind: TokenKind::OpenParen,
        }) = self.peek()
            && (cur_span.start == 0
                || ["\n", "\r"].contains(&&self.source[cur_span.start - 1..cur_span.start]))
            && let Some(Token {
                span: next_span,
                kind: TokenKind::Symbol,
            }) = self.tokens.get(self.position + 1)
            && ["defun", "defun-q", "defun-q-list-ref", "defun-q-list-set"].contains(
                &self.source[next_span.start..next_span.end]
                    .to_lowercase()
                    .as_str(),
            )
        {
            true
        } else {
            false
        }
    }

    fn parse(mut self) -> Ast {
        let mut items = Vec::new();
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::CloseParen => self.errors.push(ParseError::new(
                    token.span,
                    ParseErrorKind::UnmatchedCloseParen,
                )),
                TokenKind::Dot => self
                    .errors
                    .push(ParseError::new(token.span, ParseErrorKind::MisplacedDot)),
                TokenKind::Comment
                | TokenKind::BlockComment
                | TokenKind::Newline
                | TokenKind::Whitespace => {
                    self.advance();
                }

                TokenKind::Error(ErrorKind::UnterminatedBlockComment) => self.errors.push(
                    ParseError::new(token.span, ParseErrorKind::UnterminatedBlockComment),
                ),
                _ => items.push(self.parse_form(false)),
            }
        }
        Ast {
            root: Node {
                span: (0..self.source.len()).into(),
                kind: NodeKind::Root(items),
            },
            errors: self.errors,
        }
    }

    fn parse_form(&mut self, in_list: bool) -> Node {
        // Unwrap is ok here. The token has been checked for existence before calling parse_form.
        let Token { span, kind } = self.peek().unwrap();
        let span = *span;
        match kind {
            TokenKind::OpenParen => self.parse_list(),
            TokenKind::Quote => self.parse_quote(in_list),
            TokenKind::String => {
                self.advance();
                Node::new(span, NodeKind::String)
            }
            TokenKind::Error(ErrorKind::UnterminatedString) => {
                self.advance();
                self.errors
                    .push(ParseError::new(span, ParseErrorKind::UnterminatedString));
                Node::new(span, NodeKind::String)
            }
            TokenKind::Real => {
                self.advance();
                Node::new(span, NodeKind::Real)
            }
            TokenKind::Integer => {
                self.advance();
                let kind = if span.slice(self.source).parse::<i32>().is_ok() {
                    NodeKind::Integer
                } else {
                    NodeKind::Real
                };
                Node::new(span, kind)
            }
            TokenKind::Symbol => {
                self.advance();
                let text = span.slice(self.source).to_lowercase();
                let kind = if text.eq_ignore_ascii_case("nil") {
                    NodeKind::Nil
                } else if text.eq_ignore_ascii_case("t") {
                    NodeKind::True
                } else {
                    NodeKind::Symbol
                };
                Node::new(span, kind)
            }
            TokenKind::Dot => {
                self.advance();
                Node::new(span, NodeKind::Dot)
            }
            TokenKind::CloseParen
            | TokenKind::Comment
            | TokenKind::BlockComment
            | TokenKind::Newline
            | TokenKind::Whitespace
            | TokenKind::Error(ErrorKind::UnterminatedBlockComment) => {
                unreachable!("Skipped in `parse`")
            }
        }
    }

    fn parse_list(&mut self) -> Node {
        todo!()
    }

    fn validate_dots(&mut self, list: &[Node]) {
        let Some(idx) = list.iter().position(|n| n.kind == NodeKind::Dot) else {
            // No dots. Exit early.
            return;
        };
        if idx == 0 || idx == list.len() - 1 {
            // Dot as first or last item.
            self.errors.push(ParseError::new(
                list[idx].span,
                ParseErrorKind::MisplacedDot,
            ))
        } else if list.len() > idx + 2 {
            // Dot not penultimate item.
            let extras = &list[idx + 2..];
            self.errors.push(ParseError::new(
                Span {
                    start: extras[0].span.start,
                    end: extras[list.len() - 1].span.end,
                },
                ParseErrorKind::ExtraCdrsInDottedPair,
            ))
        }
    }

    fn parse_quote(&mut self, in_list: bool) -> Node {
        let quote_span = self.advance().span;
        let missing = match self.peek() {
            None
            | Some(Token {
                kind: TokenKind::CloseParen,
                ..
            }) => true,
            _ => in_list && self.at_recover_point(),
        };
        let child = if missing {
            self.errors.push(ParseError::new(
                Span {
                    start: quote_span.end,
                    end: quote_span.end,
                },
                ParseErrorKind::QuoteMissingOperand,
            ));
            Node::new(
                Span {
                    start: quote_span.end,
                    end: quote_span.end,
                },
                NodeKind::Missing,
            )
        } else {
            self.parse_form(in_list)
        };
        Node::new(
            Span {
                start: quote_span.start,
                end: child.span.end,
            },
            NodeKind::Quote(Box::new(child)),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    root: Node,
    errors: Vec<ParseError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    span: Span,
    kind: NodeKind,
}

impl Node {
    fn root_count(&self) -> usize {
        match &self.kind {
            NodeKind::Root(v) => 1 + v.iter().map(|n| n.root_count()).sum::<usize>(),
            NodeKind::List(v) => v.iter().map(|n| n.root_count()).sum::<usize>(),
            NodeKind::Quote(n) => n.root_count(),
            _ => 0,
        }
    }

    pub fn children(&self) -> &[Node] {
        match &self.kind {
            NodeKind::Root(v) | NodeKind::List(v) => v.as_slice(),
            NodeKind::Quote(n) => std::slice::from_ref(n.as_ref()),
            _ => &[],
        }
    }

    fn new(span: Span, kind: NodeKind) -> Self {
        Self { span, kind }
    }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ParseError {
    span: Span,
    kind: ParseErrorKind,
}
impl ParseError {
    fn new(span: Span, kind: ParseErrorKind) -> Self {
        Self { span, kind }
    }
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
