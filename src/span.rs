#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub length: usize,
}
