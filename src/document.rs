mod ast;
mod lexer;
mod parser;

use ast::Ast;

#[derive(Debug, Clone)]
pub struct Document {
    text: String,
    ast: Ast,
}

impl Document {
    pub fn new(text: String) -> Self {
        let mut doc = Self {
            text,
            ast: Ast::new(),
        };
        doc.process();
        doc
    }

    pub fn update(&mut self, new_text: String) {
        self.text = new_text;
        self.process();
    }

    fn process(&mut self) {
        let tokens = lexer::tokenize(&self.text);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Span {
    pub start: usize,
    pub length: usize,
}

const SPECIAL_FORMS: [&str; 18] = [
    "AND", "COMMAND", "COND", "DEFUN", "DEFUN-Q", "FOREACH", "FUNCTION", "IF", "LAMBDA", "OR",
    "PROGN", "QUOTE", "REPEAT", "SETQ", "TRACE", "UNTRACE", "VLAX-FOR", "WHILE",
];
