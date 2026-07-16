/// Index or size of a string in bytes.
///
/// Users have to ensure that it always points at a valid starting byte of a UTF-8 char.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct TextIndex {
    raw: usize,
}

impl TextIndex {
    /// Create a TextIndex from the length in bytes of a UTF-8 character.
    pub fn from_utf8_len(c: char) -> Self {
        Self::from(c.len_utf8())
    }

    pub fn as_index(&self) -> usize {
        self.raw
    }

    /// Move the index by the given amount.
    pub fn offset<T: Into<TextIndex>>(&mut self, other: T) {
        self.raw += other.into().raw;
    }
}

impl From<usize> for TextIndex {
    fn from(raw: usize) -> Self {
        Self { raw }
    }
}

impl std::ops::Add<TextIndex> for TextIndex {
    type Output = TextIndex;

    fn add(self, other: TextIndex) -> Self::Output {
        Self::from(self.raw + other.raw)
    }
}

impl std::ops::Sub<TextIndex> for TextIndex {
    type Output = TextIndex;

    fn sub(self, other: TextIndex) -> Self::Output {
        Self::from(self.raw - other.raw)
    }
}
