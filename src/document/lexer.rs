use std::{iter::Peekable, str::CharIndices};

use super::Span;
use tracing::error;

pub fn tokenize(source: &str) -> Vec<Token> {
    let it = source.char_indices().peekable();
    Lexer::new(source, it).tokens()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub(crate) span: Span,
    pub(crate) kind: TokenKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    OpenParen,
    CloseParen,
    String,
    Real,
    Integer,
    Symbol,
    Dot,
    Quote,
    Comment,
    BlockComment,
    Newline,
    Whitespace,
    Error(ErrorKind),
}

impl TokenKind {
    pub fn is_trivial(&self) -> bool {
        [
            Self::Comment,
            Self::BlockComment,
            Self::Whitespace,
            Self::Newline,
        ]
        .contains(self)
    }
}

impl From<&str> for TokenKind {
    fn from(value: &str) -> Self {
        let first = value.chars().next().unwrap_or(' '); // Space is ok for empty string. Using this to check for numbers.
        match value {
            "." => Self::Dot,
            _ if "-+1234567890".contains(first) => {
                if value == "+" || value == "-" {
                    return Self::Symbol;
                }
                let mut dots = 0;
                let mut exp = 0;
                let mut prev = first;
                for ch in value.chars().skip(1) {
                    match ch {
                        '.' => {
                            dots += 1;
                            // Decimal powers aren't permitted
                            if exp == 1 {
                                return Self::Symbol;
                            }
                            if dots > 1 {
                                return Self::Symbol;
                            }
                        }
                        'e' | 'E' => {
                            exp += 1;
                            if exp > 1 {
                                return Self::Symbol;
                            }
                        }
                        '+' | '-' if "eE".contains(prev) => {}
                        c if !c.is_ascii_digit() => return Self::Symbol,
                        _ => {}
                    }
                    prev = ch;
                }
                if let Some((before, _)) = value.split_once('.')
                    && !before.chars().any(|c| c.is_ascii_digit())
                {
                    return Self::Symbol;
                }
                if let Some((before, after)) = value.to_lowercase().split_once('e')
                    && !(before.chars().any(|c| c.is_ascii_digit())
                        && after.chars().any(|c| c.is_ascii_digit()))
                {
                    return Self::Symbol;
                }
                if dots == 1 || exp == 1 {
                    Self::Real
                } else {
                    Self::Integer
                }
            }
            _ => Self::Symbol,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    UnterminatedString,
    UnterminatedBlockComment,
}

struct Lexer<'a> {
    source: &'a str,
    it: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, it: Peekable<CharIndices<'a>>) -> Self {
        Self { source, it }
    }

    fn tokens(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some('\r') | Some('\n') => self.newline(&mut tokens),
                Some(' ') | Some('\t') => self.whitespace(&mut tokens),
                Some('(') | Some(')') | Some('\'') => self.atom(&mut tokens),
                Some('"') => self.string(&mut tokens),
                Some(';') => self.comment(&mut tokens),
                _ => self.word(&mut tokens),
            }
        }
        tokens
    }

    fn atom(&mut self, tokens: &mut Vec<Token>) {
        let (index, atom) = self.next();
        let step = self.next_start();
        let span = (index..step).into();
        let kind = match atom {
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '\'' => TokenKind::Quote,
            _ => {
                error!("Tried to process {atom} as atom.");
                return;
            }
        };
        tokens.push(Token { span, kind })
    }

    fn newline(&mut self, tokens: &mut Vec<Token>) {
        let (start, chr) = self.next();
        if chr == '\r' && self.peek() == Some('\n') {
            // CRLF ending
            let _ = self.next();
        }
        let end = self.next_start();
        let span = (start..end).into();
        let kind = TokenKind::Newline;
        tokens.push(Token { span, kind });
    }

    fn whitespace(&mut self, tokens: &mut Vec<Token>) {
        let (start, _) = self.next();
        while [Some(' '), Some('\t')].contains(&self.peek()) {
            let _ = self.next();
        }
        let end = self.next_start();
        tokens.push(Token {
            span: (start..end).into(),
            kind: TokenKind::Whitespace,
        });
    }

    fn comment(&mut self, tokens: &mut Vec<Token>) {
        let (start, _) = self.next(); // Comment start, ;
        match self.peek() {
            Some('|') => self.block_comment(start, tokens),
            _ => self.line_comment(start, tokens),
        }
    }

    fn line_comment(&mut self, start: usize, tokens: &mut Vec<Token>) {
        while ![Some('\n'), Some('\r'), None].contains(&self.peek()) {
            let _ = self.next();
        }
        let end = self.next_start();
        tokens.push(Token {
            span: (start..end).into(),
            kind: TokenKind::Comment,
        });
    }

    fn block_comment(&mut self, start: usize, tokens: &mut Vec<Token>) {
        let _start_pipe = self.next(); // Pipe from ;| opening.
        loop {
            match self.peek() {
                Some('|') => {
                    let _pipe = self.next();
                    // End of block comment.
                    if let Some(';') = self.peek() {
                        let _semicolon = self.next();
                        let end = self.next_start();
                        let span = (start..end).into();
                        let kind = TokenKind::BlockComment;
                        tokens.push(Token { span, kind });
                        return;
                    }
                }
                None => {
                    let end = self.next_start();
                    let span = (start..end).into();
                    let kind = TokenKind::Error(ErrorKind::UnterminatedBlockComment);
                    tokens.push(Token { span, kind });
                    return;
                }
                _ => {
                    let _ = self.next();
                }
            }
        }
    }

    fn string(&mut self, tokens: &mut Vec<Token>) {
        let (start, _) = self.next();
        let mut escape_next = false;
        loop {
            match self.peek() {
                None => {
                    let span = (start..self.next_start()).into();
                    let kind = TokenKind::Error(ErrorKind::UnterminatedString);
                    tokens.push(Token { span, kind });
                    return;
                }
                Some('"') if !escape_next => {
                    let _close_quote = self.next();
                    let end = self.next_start();
                    let span = (start..end).into();
                    let kind = TokenKind::String;
                    tokens.push(Token { span, kind });
                    return;
                }
                Some(chr) => {
                    if escape_next && chr == '\\' {
                        escape_next = false;
                    } else {
                        escape_next = chr == '\\';
                    }
                    let _ = self.next();
                }
            }
        }
    }

    fn word(&mut self, tokens: &mut Vec<Token>) {
        let delimiters = [
            Some(' '),
            Some('\t'),
            Some('\n'),
            Some('\r'),
            Some('\''),
            Some('"'),
            Some(';'),
            Some('('),
            Some(')'),
            None,
        ];
        let (start, _) = self.next();
        while !delimiters.contains(&self.peek()) {
            let _ = self.next();
        }
        let end = self.next_start();
        let span = (start..end).into();
        let kind = self.source[start..end].into();
        tokens.push(Token { span, kind });
    }

    fn peek(&mut self) -> Option<char> {
        self.it.peek().map(|(_, c)| *c)
    }

    fn next(&mut self) -> (usize, char) {
        match self.it.next() {
            Some(v) => v,
            None => {
                error!("Tried calling next without peeking first.");
                (
                    self.source.len(),
                    self.source.chars().last().unwrap_or('\n'),
                )
            }
        }
    }

    fn next_start(&mut self) -> usize {
        self.it.peek().map_or(self.source.len(), |(i, _)| *i)
    }
}

#[cfg(test)]
// Note: Most tests generated by Claude. Modified by me as items needed to be changed.
mod tests {
    use super::*;
    use ErrorKind::*;
    use TokenKind::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).into_iter().map(|t| t.kind).collect()
    }

    // --- structure ---
    #[test]
    fn test_empty_input() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(kinds(" \t\r\n "), [Whitespace, Newline, Whitespace]);
    }

    #[test]
    fn test_parens() {
        assert_eq!(kinds("()"), [OpenParen, CloseParen]);
    }

    #[test]
    fn test_carriage_return_only() {
        assert_eq!(kinds("a\rb"), [Symbol, Newline, Symbol]);
    }

    #[test]
    fn test_nested() {
        assert_eq!(
            kinds("(a (b))"),
            [
                OpenParen, Symbol, Whitespace, OpenParen, Symbol, CloseParen, CloseParen
            ]
        );
    }

    #[test]
    fn test_quote_list() {
        assert_eq!(kinds("'(a)"), [Quote, OpenParen, Symbol, CloseParen]);
    }

    #[test]
    fn test_dotted_pair() {
        assert_eq!(
            kinds("(a . b)"),
            [
                OpenParen, Symbol, Whitespace, Dot, Whitespace, Symbol, CloseParen
            ]
        );
    }

    #[test]
    fn test_leading_tab() {
        assert_eq!(kinds("\ta"), [Whitespace, Symbol])
    }

    #[test]
    fn test_leading_space() {
        assert_eq!(kinds(" a"), [Whitespace, Symbol])
    }

    // --- numbers ---
    #[test]
    fn test_integer() {
        assert_eq!(kinds("42"), [Integer]);
    }

    #[test]
    fn test_negative_integer() {
        assert_eq!(kinds("-7"), [Integer]);
    }

    #[test]
    fn test_real() {
        assert_eq!(kinds("3.4"), [Real]);
    }

    #[test]
    fn test_negative_real() {
        assert_eq!(kinds("-2.5"), [Real]);
    }

    #[test]
    fn test_real_exponent() {
        assert_eq!(kinds("1.5e-3"), [Real]);
    }

    #[test]
    fn test_no_numbers_after_decimal() {
        assert_eq!(kinds("3."), [Real])
    }

    #[test]
    fn test_lone_minus_is_symbol() {
        assert_eq!(
            kinds("(- 5 2)"),
            [
                OpenParen, Symbol, Whitespace, Integer, Whitespace, Integer, CloseParen
            ]
        );
    }

    #[test]
    fn test_dot_between_ints_is_dot() {
        assert_eq!(
            kinds("(1 . 2)"),
            [
                OpenParen, Integer, Whitespace, Dot, Whitespace, Integer, CloseParen
            ]
        );
    }

    #[test]
    fn test_one_plus_is_symbol() {
        assert_eq!(kinds("1+"), [Symbol]);
    }

    #[test]
    fn test_one_minus_is_symbol() {
        assert_eq!(kinds("1-"), [Symbol]);
    }

    #[test]
    fn test_bad_exponent_sign_location() {
        assert_eq!(kinds("1e3-5"), [Symbol])
    }

    // --- symbols ---
    #[test]
    fn test_plain_symbol() {
        assert_eq!(kinds("setq"), [Symbol]);
    }

    #[test]
    fn test_case_is_preserved() {
        assert_eq!(kinds("SetQ"), [Symbol]);
    }

    #[test]
    fn test_command_prefix() {
        assert_eq!(kinds("c:foo"), [Symbol]);
    }

    #[test]
    fn test_special_var_stars() {
        assert_eq!(kinds("*error*"), [Symbol]);
    }

    #[test]
    fn test_operators_are_symbols() {
        assert_eq!(
            kinds("(<= a b)"),
            [
                OpenParen, Symbol, Whitespace, Symbol, Whitespace, Symbol, CloseParen
            ]
        );
    }

    #[test]
    fn test_nil_and_t_are_symbols() {
        assert_eq!(kinds("nil T"), [Symbol, Whitespace, Symbol]);
    }

    #[test]
    fn test_symbol_start_with_number() {
        assert_eq!(kinds("3dc"), [Symbol]);
    }

    #[test]
    fn test_no_digits() {
        assert_eq!(kinds("-."), [Symbol])
    }

    #[test]
    fn test_no_mantissa() {
        assert_eq!(kinds("+e12"), [Symbol])
    }

    #[test]
    fn test_no_mantissa_or_exponent() {
        assert_eq!(kinds("-e"), [Symbol])
    }

    #[test]
    fn test_no_exponent() {
        assert_eq!(kinds("6e"), [Symbol])
    }

    #[test]
    fn test_no_exponent_with_sign() {
        assert_eq!(kinds("6e+"), [Symbol])
    }

    #[test]
    fn test_no_decimal_exponent() {
        assert_eq!(kinds("6e1.5"), [Symbol])
    }

    // --- strings ---
    #[test]
    fn test_simple_string() {
        assert_eq!(kinds(r#""hi""#), [String]);
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(kinds(r#""""#), [String]);
    }

    #[test]
    fn test_escaped_quote() {
        assert_eq!(kinds(r#""a\"b""#), [String]);
    }

    #[test]
    fn test_escaped_backslash_then_close() {
        assert_eq!(kinds(r#""a\\" x"#), [String, Whitespace, Symbol]);
    }

    #[test]
    fn test_paren_inside_string() {
        assert_eq!(kinds(r#"("(")"#), [OpenParen, String, CloseParen]);
    }

    #[test]
    fn test_semicolon_inside_string() {
        assert_eq!(kinds(r#""a;b""#), [String]);
    }

    #[test]
    fn test_multiline_string() {
        assert_eq!(kinds("\"a\nb\""), [String]);
    }

    #[test]
    fn test_unterminated_string() {
        assert_eq!(
            tokenize("\"abc"),
            [Token {
                span: (0..4).into(),
                kind: Error(UnterminatedString)
            }]
        );
    }

    #[test]
    fn test_string_ending_in_lone_backslash_is_unterminated() {
        assert_eq!(kinds("\"abc\\\""), [Error(UnterminatedString)]);
    }

    // --- comments ---
    #[test]
    fn test_line_comment() {
        assert_eq!(kinds("; hi\nx"), [Comment, Newline, Symbol]);
    }

    #[test]
    fn test_line_comment_at_eof() {
        assert_eq!(kinds("; hi"), [Comment]);
    }

    #[test]
    fn test_line_comment_crlf() {
        assert_eq!(kinds("; a\r\nb"), [Comment, Newline, Symbol]);
    }

    #[test]
    fn test_paren_in_line_comment() {
        assert_eq!(kinds("; )"), [Comment]);
    }

    #[test]
    fn test_block_comment() {
        assert_eq!(kinds(";| x |; y"), [BlockComment, Whitespace, Symbol]);
    }

    #[test]
    fn test_block_comment_inline() {
        assert_eq!(
            kinds("(a ;| x |; b)"),
            [
                OpenParen,
                Symbol,
                Whitespace,
                BlockComment,
                Whitespace,
                Symbol,
                CloseParen
            ]
        );
    }

    #[test]
    fn test_block_comment_multiline() {
        assert_eq!(kinds(";| a\nb |;"), [BlockComment]);
    }

    #[test]
    fn test_block_comment_hides_line_comment_marker() {
        assert_eq!(kinds(";| ; |; x"), [BlockComment, Whitespace, Symbol]);
    }

    #[test]
    fn test_block_comment_hides_pipe() {
        assert_eq!(kinds(";| | ; |;"), [BlockComment]);
    }

    #[test]
    fn test_short_unterminated_block_comment() {
        assert_eq!(kinds(";|;"), [Error(UnterminatedBlockComment)]);
    }

    #[test]
    fn test_unterminated_block_comment() {
        assert_eq!(
            tokenize(";| oops"),
            [Token {
                span: (0..7).into(),
                kind: Error(UnterminatedBlockComment)
            }]
        );
    }

    // --- spans (start + length, in BYTES) ---
    #[test]
    fn test_spans_simple() {
        assert_eq!(
            tokenize("(ab)"),
            [
                Token {
                    span: (0..1).into(),
                    kind: OpenParen
                },
                Token {
                    span: (1..3).into(),
                    kind: Symbol
                },
                Token {
                    span: (3..4).into(),
                    kind: CloseParen
                },
            ]
        );
    }

    #[test]
    fn test_spans_with_whitespace() {
        let t = tokenize("a   b");
        assert_eq!((t[2].span.start, t[2].span.end), (4, 5));
    }

    #[test]
    fn test_spans_multibyte_utf8() {
        // 'é' is 2 bytes, so the string token is 4 bytes and 'x' starts at byte 5
        let t = tokenize("\"é\" x");
        assert_eq!(t[0].span, (0..4).into());
        assert_eq!(t[1].span, (4..5).into());
        assert_eq!(t[2].span, (5..6).into());
    }

    #[test]
    fn test_spans_ordered_and_in_bounds() {
        let src = "(defun c:f (/ a) ; c\n (setq a 1.5 s \"x\"))";
        let tokens = tokenize(src);
        for t in tokens {
            assert!(t.span.end <= src.len());
        }
    }

    // --- robustness ---
    #[test]
    fn test_never_panics_on_garbage() {
        for s in [
            "\"", ";|", "\\", ")(", "'", ".", "\u{0}", "\r\n\r\n", "|;", "é", "-", "((((",
        ] {
            let _ = tokenize(s);
        }
    }
}
