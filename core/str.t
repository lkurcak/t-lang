//! Strings: text in UTF-8, seen through a reference.

use crate::cmp::{Ord, Ordering};
use crate::iter::Iterator;
use crate::option::Option;
use crate::slice::Split;

/// `str`: a string, bytes that are UTF-8, seen through a shared reference,
/// `&str`, which knows how many. Like a slice, it's never a value of its
/// own; unlike one, it's never behind a `&mut`, as changing its bytes
/// could make them not UTF-8.
///
/// That they are is what a `str` adds to its bytes, and what's trusted is
/// what keeps it so: `as_bytes`, which only lends them to be read, and
/// `from_utf8_unchecked`, whose caller promises it. A string literal is
/// UTF-8 too, as the source is. The rest is safe code over them.
impl str {
    /// Its bytes, its UTF-8: as many as it has.
    #[intrinsic]
    pub fn as_bytes(&self) -> { b: &[u8] | b.len() == self.len() };

    /// How many bytes it has: not how many characters. It's a measure, of
    /// the reference, as a slice's is.
    #[intrinsic]
    #[measure]
    pub fn len(&self) -> usize;

    /// Has it none?
    #[measure]
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Does a character start at byte `i`, or is `i` its end? Then it can
    /// be cut there.
    pub fn is_char_boundary(&self, i: usize) -> bool {
        match self.as_bytes().get(i) {
            Option::Some(b) => !is_continuation(*b),
            Option::None => i == self.len(),
        }
    }

    /// The bytes before `mid` and those from it on, or `None` if no
    /// character starts at `mid` and it isn't the end.
    pub fn split_at<'a>(&'a self, mid: usize) -> Option<StrSplit<'a>> {
        if !self.is_char_boundary(mid) {
            return Option::None;
        }
        let bytes: &'a [u8] = self.as_bytes();
        match bytes.split_at(mid) {
            // SAFETY: UTF-8 cut where a character starts is UTF-8 on
            // either side.
            Option::Some(Split { head, tail }) => unsafe {
                Option::Some(StrSplit {
                    head: from_utf8_unchecked(head),
                    tail: from_utf8_unchecked(tail),
                })
            },
            Option::None => Option::None,
        }
    }

    /// Bytes `lo` up to, not including, `hi`, or `None` if they aren't
    /// where characters start, or the end, or `lo` is past `hi`.
    pub fn get<'a>(&'a self, lo: usize, hi: usize) -> Option<&'a str> {
        if lo > hi {
            return Option::None;
        }
        let head: &'a str = match self.split_at(hi) {
            Option::Some(s) => s.head,
            Option::None => return Option::None,
        };
        match head.split_at(lo) {
            Option::Some(s) => Option::Some(s.tail),
            Option::None => Option::None,
        }
    }

    /// Its characters, front to back.
    pub fn chars(&self) -> Chars { Chars { bytes: self.as_bytes(), at: 0 } }
}

/// Bytes `b`, as a string, if they're UTF-8, or `None`.
pub fn from_utf8(b: &[u8]) -> Option<&str> {
    let n = b.len();
    let mut at = 0usize;
    while at < n {
        match decode(b, at) {
            Option::Some(d) => at = d.next,
            Option::None => return Option::None,
        }
    }
    // SAFETY: every byte is in a character `decode` found.
    Option::Some(unsafe { from_utf8_unchecked(b) })
}

/// Bytes `b`, as a string.
///
/// # Safety
///
/// They're UTF-8.
#[intrinsic]
pub fn from_utf8_unchecked(b: &[u8]) -> { s: &str | s.len() == b.len() } with unsafe;

/// What `split_at` gives: the string before the index it's given, and
/// from it on.
#[derive(Copy, Discard)]
pub struct StrSplit<'a> { pub head: &'a str, pub tail: &'a str }

/// What `chars` gives: each character of `bytes` from `at` on.
#[derive(Discard)]
pub struct Chars<'a> { bytes: &'a [u8], at: usize }

/// Safe code over `decode`. A `str`'s bytes are UTF-8, so it stops only
/// at their end; were they not, it would stop at the first that isn't.
impl<'a> Iterator for Chars<'a> {
    type Item = char;
    effect e = {};

    fn next(&mut self) -> Option<char> {
        match decode(self.bytes, self.at) {
            Option::Some(d) => {
                self.at = d.next;
                Option::Some(d.c)
            },
            Option::None => Option::None,
        }
    }
}

/// Bytes in their order, the first that differ deciding: the order of
/// their characters' numbers, as UTF-8 keeps it.
impl Ord for str {
    fn cmp(&self, other: &str) -> Ordering {
        let a = self.as_bytes();
        let b = other.as_bytes();
        let n = a.len();
        let m = b.len();
        let mut i = 0usize;
        while i < n && i < m {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
            i = i + 1;
        }
        n.cmp(&m)
    }
}

/// Is `b` one of the bytes after a character's first in UTF-8, `10xxxxxx`?
fn is_continuation(b: u8) -> bool { b >= 0x80 && b <= 0xBF }

/// What `decode` finds: a character, and where the next one starts.
#[derive(Copy, Discard)]
struct Decoded { c: char, next: usize }

/// The character whose UTF-8 starts at byte `at` of `b`, or `None` if
/// there's none there: if `at` is the end, or what's there isn't UTF-8.
///
/// UTF-8 is, for each character, one byte `0xxxxxxx` for the numbers below
/// `80`, or a byte `110xxxxx`, `1110xxxx` or `11110xxx` and then one, two
/// or three `10xxxxxx`: the number's bits, in as few bytes as hold them.
/// So a character written in more (*overlong*), a surrogate, and anything
/// past `10FFFF` aren't UTF-8.
///
/// Each byte's bits are found by subtracting what's above them, and put
/// together with `+`, as they don't overlap: arithmetic the checker proves
/// can't overflow without splitting into cases, as `&` and `|` would.
fn decode(b: &[u8], at: usize) -> Option<Decoded> {
    let n = b.len();
    if at >= n {
        return Option::None;
    }
    let x = b[at] as u32;
    if x < 0x80 {
        return Option::Some(Decoded { c: x as char, next: at + 1 });
    }
    // A continuation byte can't come first, `C0` and `C1` could only start
    // an overlong two, and `F5` and up something past `10FFFF`.
    if x < 0xC2 || x > 0xF4 {
        return Option::None;
    }
    if at >= n - 1 {
        return Option::None;
    }
    let c1 = match continuation(b, at + 1) {
        Option::Some(c) => c.bits,
        Option::None => return Option::None,
    };
    if x < 0xE0 {
        let v = (x - 0xC0) * 0x40 + c1;
        return Option::Some(Decoded { c: v as char, next: at + 2 });
    }
    if at + 1 >= n - 1 {
        return Option::None;
    }
    let c2 = match continuation(b, at + 2) {
        Option::Some(c) => c.bits,
        Option::None => return Option::None,
    };
    if x < 0xF0 {
        let v = (x - 0xE0) * 0x1000 + c1 * 0x40 + c2;
        if v < 0x800 || (v >= 0xD800 && v <= 0xDFFF) {
            return Option::None;
        }
        return Option::Some(Decoded { c: v as char, next: at + 3 });
    }
    if at + 2 >= n - 1 {
        return Option::None;
    }
    let c3 = match continuation(b, at + 3) {
        Option::Some(c) => c.bits,
        Option::None => return Option::None,
    };
    let v = (x - 0xF0) * 0x40000 + c1 * 0x1000 + c2 * 0x40 + c3;
    if v < 0x10000 || v > 0x10FFFF {
        return Option::None;
    }
    Option::Some(Decoded { c: v as char, next: at + 4 })
}

/// The six bits of a continuation byte, `10xxxxxx`.
#[derive(Copy, Discard)]
struct Six { bits: { c: u32 | c <= 0x3F } }

/// The six bits of byte `i` of `b`, if it's a continuation byte.
fn continuation(b: &[u8], i: { i: usize | i < b.len() }) -> Option<Six> {
    let y = b[i];
    if y < 0x80 || y > 0xBF {
        return Option::None;
    }
    Option::Some(Six { bits: (y - 0x80) as u32 })
}
