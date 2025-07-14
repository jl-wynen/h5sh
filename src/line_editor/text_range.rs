use std::ops::Index;

use super::text_index::TextIndex;

/// Range of bytes in a string.
///
/// Users have to ensure that it always points at a valid starting byte of a UTF-8 char.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct TextRange {
    start: TextIndex,
    end: TextIndex,
}

impl TextRange {
    /// Create an empty TextRange starting at a given index.
    pub fn start_new(start: TextIndex) -> Self {
        Self { start, end: start }
    }

    pub fn start(&self) -> TextIndex {
        self.start
    }

    pub fn end(&self) -> TextIndex {
        self.end
    }

    /// Extend the TextRange to the given end.
    pub fn extend_to(&mut self, end: TextIndex) {
        if end < self.start {
            panic!("end < start");
        }
        self.end = end;
    }

    /// Extend the beginning of the TextRange to the given start.
    pub fn extend_backwards_to(&mut self, start: TextIndex) {
        self.start = start;
    }

    /// Move the start n bytes.
    pub fn skip_n(&mut self, n: usize) {
        self.start.offset(TextIndex::from(n));
        self.end = self.end.max(self.start);
    }

    /// Return true if the range contains no elements.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Return true of the index is within this range.
    pub fn contains_or_end(&self, index: TextIndex) -> bool {
        self.start <= index && index <= self.end
    }
}

impl From<(usize, usize)> for TextRange {
    fn from((start, end): (usize, usize)) -> Self {
        Self {
            start: TextIndex::from(start),
            end: TextIndex::from(end),
        }
    }
}

impl Index<TextRange> for str {
    type Output = str;

    fn index(&self, index: TextRange) -> &Self::Output {
        &self[index.start.as_index()..index.end.as_index()]
    }
}
