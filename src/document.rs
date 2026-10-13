mod ast;
mod lexer;
mod parser;

#[derive(Debug, Clone, Default)]
pub struct Document {
    text: String,
    line_starts: Vec<usize>, // Byte positions of each line beginning
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
    pub fn to_position(&self, offset: usize) -> Position {
        let line = self
            .line_starts
            .partition_point(|pos| *pos <= offset)
            .saturating_sub(1); // -1 should be ok but to ensure we don't underflow let's saturating_sub.
        let line_start = self.line_starts[line];
        let char_offset = self.text[line_start..offset]
            .chars()
            .map(|chr| chr.len_utf16())
            .sum::<usize>();
        (line_start, char_offset).into()
    }

    /// Calculate the offset into the text of a `Position`
    pub fn to_offset(&self, position: Position) -> usize {
        let (line, offset) = position.into();
        let start = self.line_starts[line];
        let end = (start + 1).min(self.text.len());
        let mut rem = 0;
        for (ind, chr) in self.text[start..end].char_indices() {
            if rem >= offset {
                return start + ind;
            }
            rem += chr.len_utf16();
        }
        end
    }

    fn process(&mut self) {
        self.get_line_starts();
        let tokens = lexer::tokenize(&self.text);
    }

    fn get_line_starts(&mut self) {
        let mut line_starts = vec![0];
        for (index, chr) in self.text.char_indices() {
            if chr == '\n' {
                line_starts.push(index + 1);
            }
        }
        self.line_starts = line_starts
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Span {
    pub start: usize,
    pub length: usize,
}

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
