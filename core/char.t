//! Characters: Unicode scalar values.

use crate::option::Option;
use crate::str;

/// A `char` is a Unicode scalar value: a number from 0 to `D7FF`, or from
/// `E000` to `10FFFF`. The ones in between, the surrogates, are halves of
/// UTF-16 pairs, not characters. `c as u32` is its number, and `n as char`
/// the character numbered `n`, which panics if there's none, unless it's
/// proved there is.
impl char {
    /// The character numbered `n`, or `None` if there's none.
    export fn from_u32(n: u32) -> Option<char> {
        if n <= 0x10FFFF && (n < 0xD800 || n > 0xDFFF) {
            Option::Some(n as char)
        } else {
            Option::None
        }
    }

    /// How many bytes it takes in UTF-8: 1 to 4.
    export fn len_utf8(self) -> usize {
        let n = self as u32;
        if n < 0x80 { 1 } else if n < 0x800 { 2 } else if n < 0x10000 { 3 } else { 4 }
    }

    /// Writes it, in UTF-8, at the start of `out`, and gives what it
    /// wrote, or `None` if `out` is too short for it.
    export fn encode_utf8(self, out: &mut [u8]) -> Option<&str> {
        let n = self as u32;
        let bytes: &mut [u8] = match out.split_at_mut(self.len_utf8()) {
            Option::Some(split) => split.head,
            Option::None => return Option::None,
        };
        if n < 0x80 {
            set(bytes, 0, n as u8);
        } else if n < 0x800 {
            set(bytes, 0, ((n >> 6) & 0x1F) as u8 | 0xC0);
            set(bytes, 1, tail(n, 0));
        } else if n < 0x10000 {
            set(bytes, 0, ((n >> 12) & 0x0F) as u8 | 0xE0);
            set(bytes, 1, tail(n, 6));
            set(bytes, 2, tail(n, 0));
        } else {
            set(bytes, 0, ((n >> 18) & 0x07) as u8 | 0xF0);
            set(bytes, 1, tail(n, 12));
            set(bytes, 2, tail(n, 6));
            set(bytes, 3, tail(n, 0));
        }
        str::from_utf8(&*bytes)
    }

    /// Is it ASCII: is its number below 128?
    export fn is_ascii(self) -> bool { self as u32 < 0x80 }

    /// Is it `A` to `Z` or `a` to `z`?
    export fn is_ascii_alphabetic(self) -> bool {
        self.is_ascii_uppercase() || self.is_ascii_lowercase()
    }

    /// Is it `A` to `Z`?
    export fn is_ascii_uppercase(self) -> bool {
        match self {
            'A'..='Z' => true,
            _ => false,
        }
    }

    /// Is it `a` to `z`?
    export fn is_ascii_lowercase(self) -> bool {
        match self {
            'a'..='z' => true,
            _ => false,
        }
    }

    /// Is it `0` to `9`?
    export fn is_ascii_digit(self) -> bool {
        match self {
            '0'..='9' => true,
            _ => false,
        }
    }

    /// Is it a space, a tab, a line feed, a form feed or a carriage return?
    export fn is_ascii_whitespace(self) -> bool {
        match self {
            ' ' | '\t' | '\n' | '\u{C}' | '\r' => true,
            _ => false,
        }
    }
}

/// Byte `i` of `s` becomes `b`, if there's one.
fn set(s: &mut [u8], i: usize, b: u8) {
    match s.get_mut(i) {
        Option::Some(x) => *x = b,
        Option::None => {},
    }
}

/// A byte after the first in UTF-8: `10`, then six bits of `n`, from bit
/// `shift` up.
fn tail(n: u32, shift: u32) -> u8 { ((n >> shift) & 0x3F) as u8 | 0x80 }
