use std::str::Chars;

use super::text_index::TextIndex;
use super::text_range::TextRange;

#[derive(Debug)]
pub(super) struct Scanner<'a> {
    current: char,
    /// The byte index of the start of the current char.
    current_index: TextIndex,
    chars: Chars<'a>,
    src: &'a str,
}

impl<'a> Scanner<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut chars = src.chars();
        let current = chars.next().unwrap_or('\0');
        Self {
            current,
            current_index: TextIndex::default(),
            chars,
            src,
        }
    }

    pub fn current(&self) -> char {
        self.current
    }

    pub fn current_index(&self) -> TextIndex {
        self.current_index
    }

    pub fn is_finished(&self) -> bool {
        self.current == '\0'
    }

    pub fn eat(&mut self) -> char {
        self.current_index
            .offset(TextIndex::from_utf8_len(self.current));
        let new = read_next(&mut self.chars);
        self.current = new;
        self.current
    }
}

fn read_next(chars: &mut Chars) -> char {
    chars.next().unwrap_or('\0')
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn scanner_eat_any() {
        let src = "2y̆Å";
        let mut scanner = Scanner::new(src);
        assert_eq!(scanner.current(), '2');
        assert_eq!(scanner.current_index(), TextIndex::from(0));

        assert_eq!(scanner.eat(), 'y');
        assert_eq!(scanner.current(), 'y');
        assert_eq!(scanner.current_index(), TextIndex::from(1));

        assert_eq!(scanner.eat(), '\u{0306}');
        assert_eq!(scanner.current(), '\u{0306}');
        assert_eq!(scanner.current_index(), TextIndex::from(2));

        assert_eq!(scanner.eat(), 'Å');
        assert_eq!(scanner.current(), 'Å');
        assert_eq!(scanner.current_index(), TextIndex::from(4));

        assert_eq!(scanner.eat(), '\0');
        assert_eq!(scanner.current(), '\0');
        assert_eq!(scanner.current_index(), TextIndex::from(6));

        assert_eq!(scanner.eat(), '\0');
        assert_eq!(scanner.current(), '\0');
        assert_eq!(scanner.current_index(), TextIndex::from(7));
    }
}
