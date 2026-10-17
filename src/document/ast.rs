use crate::document::{
    Span,
    lexer::{ErrorKind, Token, TokenKind},
};

pub fn parse<'a>(tokens: &'a [Token], source: &'a str) -> Ast {
    let builder = AstBuilder::new(tokens, source);
    let ast = builder.parse();
    debug_assert!(ast.root.root_count() == 1);
    ast
}

struct AstBuilder<'a> {
    source: &'a str,
    tokens: Vec<&'a Token>,
    position: usize,
    previous_end: usize,
    errors: Vec<ParseError>,
}

impl<'a> AstBuilder<'a> {
    fn new(tokens: &'a [Token], source: &'a str) -> Self {
        let mut toks = Vec::new();
        let mut errors = Vec::new();
        for token in tokens {
            match token.kind {
                t if t.is_trivial() => {}
                TokenKind::Error(ErrorKind::UnterminatedBlockComment) => {
                    errors.push(ParseError::new(
                        token.span,
                        ParseErrorKind::UnterminatedBlockComment,
                    ));
                }
                _ => toks.push(token),
            }
        }
        Self {
            source,
            tokens: toks,
            position: 0,
            previous_end: 0,
            errors,
        }
    }

    fn peek(&self) -> Option<&&Token> {
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
        if self.peek().is_none() {
            true
        } else if let Some(Token {
            span: cur_span,
            kind: TokenKind::OpenParen,
        }) = self.peek()
            && (cur_span.start == 0
                || ["\n", "\r"].contains(&&self.source[cur_span.start - 1..cur_span.start]))
            && let Some(Token {
                span: next_span,
                kind: TokenKind::Symbol,
            }) = self.tokens.get(self.position + 1)
            && ["defun", "defun-q", "defun-q-list-ref", "defun-q-list-set"]
                .contains(&next_span.slice(self.source).to_lowercase().as_str())
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
                TokenKind::CloseParen => {
                    self.errors.push(ParseError::new(
                        token.span,
                        ParseErrorKind::UnmatchedCloseParen,
                    ));
                    self.advance();
                }
                TokenKind::Dot => {
                    let span = token.span;
                    self.errors
                        .push(ParseError::new(span, ParseErrorKind::MisplacedDot));
                    items.push(Node::new(span, NodeKind::Dot));
                    self.advance();
                }
                t if t.is_trivial() => {
                    tracing::error!("Somehow got a filtered token. {:?}", token.kind);
                    self.advance();
                }
                TokenKind::Error(ErrorKind::UnterminatedBlockComment) => {
                    tracing::error!("Somehow got a filtered token. {:?}", token.kind);
                    self.advance();
                }
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
                let text = span.slice(self.source);
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
            TokenKind::Comment
            | TokenKind::BlockComment
            | TokenKind::Newline
            | TokenKind::Whitespace
            | TokenKind::Error(ErrorKind::UnterminatedBlockComment)
            | TokenKind::CloseParen => {
                tracing::error!("Somehow got a filtered token. {kind:?}");
                self.advance();
                self.parse_form(in_list)
            }
        }
    }

    fn parse_list(&mut self) -> Node {
        let open = self.advance().span; // Absorb open bracket.
        let mut list = Vec::new();
        let mut closed = false;
        let mut end = self.previous_end;
        while let Some(token) = self.peek() {
            match token.kind {
                t if t.is_trivial() => {
                    tracing::error!("Somehow got a filtered token. {:?}", token.kind);
                    self.advance();
                }
                TokenKind::Error(ErrorKind::UnterminatedBlockComment) => {
                    tracing::error!("Somehow got a filtered token. {:?}", token.kind);
                    self.advance();
                }
                TokenKind::CloseParen => {
                    closed = true;
                    end = self.advance().span.end;
                    break;
                }
                _ if self.at_recover_point() => {
                    break;
                }
                _ => list.push(self.parse_form(true)),
            }
        }
        // EOF or unclosed bracket.
        if !closed {
            end = self.previous_end;
            self.errors.push(ParseError::new(
                (end..end).into(),
                ParseErrorKind::UnclosedList,
            ));
        } else {
            self.validate_dots(&list);
        }
        Node::new(
            Span {
                start: open.start,
                end,
            },
            NodeKind::List(list),
        )
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
                    end: extras[extras.len() - 1].span.end,
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
            _ if (in_list && self.at_recover_point()) => true,
            _ => false,
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
