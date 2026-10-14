mod ast;
mod lexer;
mod parser;

use lexer::{Token, tokenize};

/// Contents of one document.
#[derive(Debug, Clone, Default)]
pub struct Document {
    text: String,
    line_start_offsets: Vec<usize>, // Byte positions of each line start
    tokens: Vec<Token>,
}

impl Document {
    pub fn new(text: String) -> Self {
        let mut doc = Self {
            text,
            ..Default::default()
        };
        doc.process();
        doc
    }

    pub fn update(&mut self, new_text: String) {
        self.text = new_text;
        self.process();
    }

    /// Turn a character offset into a `Position`
    pub fn to_position(&self, mut offset: usize) -> Position {
        offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self
            .line_start_offsets
            .partition_point(|pos| *pos <= offset)
            .saturating_sub(1); // -1 should be ok but to ensure we don't underflow let's saturating_sub.
        let line_start = self.line_start_offsets[line];
        let char_offset = self.text[line_start..offset]
            .chars()
            .map(|chr| chr.len_utf16())
            .sum::<usize>();
        (line, char_offset).into()
    }

    /// Calculate the offset into the text of a `Position`
    pub fn to_offset(&self, position: Position) -> usize {
        let (line, offset) = position.into();
        // Are we past the end?
        if line >= self.line_start_offsets.len() {
            return self.text.len();
        }
        let start_offset = self.line_start_offsets[line];
        let next_line = self
            .line_start_offsets
            .get(line + 1) // Next index unless that pushes us past the end.
            .copied()
            .unwrap_or(self.text.len()); // End of the text if we're on the last line.
        let line_text = &self.text[start_offset..next_line];
        let line_text = line_text
            .strip_suffix('\n')
            .map(|s| s.strip_suffix('\r').unwrap_or(s))
            .unwrap_or(line_text);
        let mut rem = 0;
        for (ind, chr) in line_text.char_indices() {
            if rem >= offset {
                return start_offset + ind;
            }
            rem += chr.len_utf16();
        }
        start_offset + line_text.len()
    }

    /// Show the text within a span may panic if the slice is not valid.
    pub fn text_of_unchecked(&self, span: Span) -> &str {
        span.slice_unchecked(&self.text)
    }

    /// Show the text within a span.
    pub fn text_of(&self, span: Span) -> Option<&str> {
        span.try_slice(&self.text)
    }

    fn process(&mut self) {
        self.get_line_starts();
        self.tokens = tokenize(&self.text);
    }

    fn get_line_starts(&mut self) {
        let mut line_starts = vec![0];
        let mut r_as_newline = false;
        for (index, chr) in self.text.char_indices() {
            if chr == '\n' {
                line_starts.push(index + 1);
                r_as_newline = false;
            }
            if r_as_newline {
                line_starts.push(index); // Passed \r but didn't get a \n (mac old style)
                r_as_newline = false;
            }
            if chr == '\r' {
                r_as_newline = true;
            }
        }
        // \r is the last character.
        if r_as_newline {
            self.line_start_offsets.push(self.text.len());
        }
        self.line_start_offsets = line_starts
    }
}

/// Start and length of an item in the document.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub length: usize,
}

impl Span {
    /// Extract a subsection of text. The span must fit entirely into the text.
    pub fn slice_unchecked(self, text: &str) -> &str {
        &text[self.start..self.start + self.length]
    }

    /// Extract a subsection of text less confidently. The span must fit entirely into the text.
    pub fn try_slice(self, text: &str) -> Option<&str> {
        text.get(self.start..self.start + self.length)
    }
}

impl From<(usize, usize)> for Span {
    fn from(value: (usize, usize)) -> Self {
        Self {
            start: value.0,
            length: value.1,
        }
    }
}

/// UTF-16 position.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Position {
    pub line_number: u32,
    pub char_offset: u32,
}

impl From<(usize, usize)> for Position {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line_number: u32::try_from(value.0).unwrap_or(u32::MAX),
            char_offset: u32::try_from(value.1).unwrap_or(u32::MAX),
        }
    }
}

impl From<Position> for (usize, usize) {
    fn from(value: Position) -> Self {
        (value.line_number as usize, value.char_offset as usize)
    }
}

const SPECIAL_FORMS: [&str; 18] = [
    "AND", "COMMAND", "COND", "DEFUN", "DEFUN-Q", "FOREACH", "FUNCTION", "IF", "LAMBDA", "OR",
    "PROGN", "QUOTE", "REPEAT", "SETQ", "TRACE", "UNTRACE", "VLAX-FOR", "WHILE",
];
