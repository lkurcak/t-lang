//! Strings: text in UTF-8, seen through a reference.

use crate::cmp::{Ord, Ordering};
use crate::iter::Iterator;
use crate::option::Option;
use crate::fmt::{Display, Write};
use crate::result::Result;
use crate::slice;

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
    export fn as_bytes(&self) -> { b: &[u8] | b.len() == self.len() };

    /// How many bytes it has: not how many characters. It's a measure, of
    /// the reference, as a slice's is.
    #[intrinsic]
    #[measure]
    export fn len(&self) -> usize;

    /// Has it none?
    #[measure]
    export fn is_empty(&self) -> bool { self.len() == 0 }

    /// Does a character start at byte `i`, or is `i` its end? Then it can
    /// be cut there.
    export fn is_char_boundary(&self, i: usize) -> bool {
        match self.as_bytes().get(i) {
            Some(b) => !is_continuation(*b),
            None => i == self.len(),
        }
    }

    /// The bytes before `mid` and those from it on, or `None` if no
    /// character starts at `mid` and it isn't the end.
    export fn split_at<'a>(&'a self, mid: usize) -> Option<StrSplit<'a>> {
        if !self.is_char_boundary(mid) {
            return None;
        }
        let bytes: &'a [u8] = self.as_bytes();
        match bytes.split_at(mid) {
            // SAFETY: UTF-8 cut where a character starts is UTF-8 on
            // either side.
            Some(slice::Split { head, tail }) => unsafe {
                Some(StrSplit { head: from_utf8_unchecked(head), tail: from_utf8_unchecked(tail) })
            },
            None => None,
        }
    }

    /// Bytes `lo` up to, not including, `hi`, or `None` if they aren't
    /// where characters start, or the end, or `lo` is past `hi`.
    export fn get<'a>(&'a self, lo: usize, hi: usize) -> Option<&'a str> {
        if lo > hi {
            return None;
        }
        let head: &'a str = match self.split_at(hi) {
            Some(s) => s.head,
            None => return None,
        };
        match head.split_at(lo) {
            Some(s) => Some(s.tail),
            None => None,
        }
    }

    /// Its characters, front to back.
    export fn chars(&self) -> Chars { Chars { bytes: self.as_bytes(), at: 0 } }

    /// The pieces between the places `pat` is found, front to back: a
    /// `,` gives `"a"`, `""` and `"b"` of `"a,,b"`, and `""` and `""` of
    /// `","`. An empty string is found before and after each character.
    export fn split<'a, P: Pattern>(&'a self, pat: P) -> Split<'a, P> {
        Split { rest: self, pat, empty: false, done: false }
    }

    /// The words between whitespace, front to back, none of them empty:
    /// `"a"` and `"b"` of `" a\t b\n"`.
    export fn split_whitespace<'a>(&'a self) -> SplitWhitespace<'a> {
        SplitWhitespace { rest: self }
    }

    /// Its lines, front to back, without the `\n` that ends each, or the
    /// `\r\n`. A last line needn't end in one, and an empty string has no
    /// lines.
    export fn lines<'a>(&'a self) -> Lines<'a> { Lines { rest: self } }

    /// Without the whitespace at its start and its end, which
    /// `char::is_whitespace` says.
    export fn trim<'a>(&'a self) -> &'a str { self.trim_start().trim_end() }

    /// Without the whitespace at its start.
    export fn trim_start<'a>(&'a self) -> &'a str {
        let b = self.as_bytes();
        let mut at = 0usize;
        loop {
            match decode(b, at) {
                Some(d) => match d.c.is_whitespace() {
                    true => at = d.next,
                    false => return after(self, at),
                },
                None => return after(self, at),
            }
        }
    }

    /// Without the whitespace at its end.
    export fn trim_end<'a>(&'a self) -> &'a str {
        let b = self.as_bytes();
        let mut at = 0usize;
        let mut end = 0usize;
        loop {
            match decode(b, at) {
                Some(d) => {
                    if !d.c.is_whitespace() {
                        end = d.next;
                    }
                    at = d.next;
                },
                None => return before(self, end),
            }
        }
    }

    /// Does it start with `pat`?
    export fn starts_with<P: Pattern>(&self, pat: P) -> bool { pat.prefix_of(self).is_some() }

    /// Does it end with `pat`?
    export fn ends_with<P: Pattern>(&self, pat: P) -> bool { pat.suffix_of(self).is_some() }

    /// What follows `pat`, if it starts with it, or `None`.
    export fn strip_prefix<'a, P: Pattern>(&'a self, pat: P) -> Option<&'a str> {
        match pat.prefix_of(self) {
            Some(n) => Some(after(self, n)),
            None => None,
        }
    }

    /// What comes before `pat`, if it ends with it, or `None`.
    export fn strip_suffix<'a, P: Pattern>(&'a self, pat: P) -> Option<&'a str> {
        let Some(n) = pat.suffix_of(self) else { return None };
        if n > self.len() {
            return None;
        }
        Some(before(self, self.len() - n))
    }

    /// The byte where `pat` is first found, or `None` if it isn't.
    export fn find<P: Pattern>(&self, pat: P) -> Option<usize> {
        match pat.find_in(self, 0) {
            Some(f) => Some(f.start),
            None => None,
        }
    }

    /// Is `pat` in it anywhere?
    export fn contains<P: Pattern>(&self, pat: P) -> bool { pat.find_in(self, 0).is_some() }

    /// It, read as a `T`, or what's wrong with it as one: `"42".parse()`
    /// is `Ok(42)` of any integer type that holds 42.
    export fn parse<T: Parse>(&self) -> Result<T, T::Error> { T::parse(self) }
}

/// The bytes of `s` from `at` on. `at` is where a character starts, or the
/// end, wherever it's used; were it not, it would be the empty string.
fn after<'a>(s: &'a str, at: usize) -> &'a str {
    match s.split_at(at) {
        Some(cut) => cut.tail,
        None => "",
    }
}

/// The bytes of `s` before `at`, which is where a character starts or the
/// end, as for `after`.
fn before<'a>(s: &'a str, at: usize) -> &'a str {
    match s.split_at(at) {
        Some(cut) => cut.head,
        None => "",
    }
}

/// What a `str`'s methods look for: a `char`, or a `&str`. It's sealed, as
/// those methods count on what it finds starting and ending where
/// characters do.
export sealed trait Pattern: Copy {
    /// Where it's first found in `s`, starting at byte `from` or after.
    fn find_in(&self, s: &str, from: usize) -> Option<Found>;

    /// If `s` starts with it, how many bytes of `s` it is.
    fn prefix_of(&self, s: &str) -> Option<usize>;

    /// If `s` ends with it, how many bytes of `s` it is.
    fn suffix_of(&self, s: &str) -> Option<usize>;
}

/// Where a pattern was found: bytes `start` up to, not including, `end`.
#[derive(Copy, Debug, Discard, Eq, Send, Sync)]
export struct Found { export start: usize, export end: usize }

/// A string is found where its bytes are: as it's UTF-8, that's where a
/// character starts, and it ends where one does.
impl<'b> Pattern for &'b str {
    fn find_in(&self, s: &str, from: usize) -> Option<Found> {
        let h = s.as_bytes();
        let n = self.as_bytes();
        if n.len() > h.len() {
            return None;
        }
        let last = h.len() - n.len();
        let mut i = from;
        while i <= last {
            if bytes_at(h, n, i) {
                return Some(Found { start: i, end: i + n.len() });
            }
            if i == last {
                return None;
            }
            i = i + 1;
        }
        None
    }

    fn prefix_of(&self, s: &str) -> Option<usize> {
        let h = s.as_bytes();
        let n = self.as_bytes();
        if n.len() <= h.len() && bytes_at(h, n, 0) { Some(n.len()) } else { None }
    }

    fn suffix_of(&self, s: &str) -> Option<usize> {
        let h = s.as_bytes();
        let n = self.as_bytes();
        if n.len() <= h.len() && bytes_at(h, n, h.len() - n.len()) { Some(n.len()) } else { None }
    }
}

/// A character is found where its UTF-8 is, as a string.
impl Pattern for char {
    fn find_in(&self, s: &str, from: usize) -> Option<Found> {
        let mut buf = [0u8; 4];
        match self.encode_utf8(buf.as_mut_slice()) {
            Some(p) => Pattern::find_in(&p, s, from),
            None => None,
        }
    }

    fn prefix_of(&self, s: &str) -> Option<usize> {
        let mut buf = [0u8; 4];
        match self.encode_utf8(buf.as_mut_slice()) {
            Some(p) => Pattern::prefix_of(&p, s),
            None => None,
        }
    }

    fn suffix_of(&self, s: &str) -> Option<usize> {
        let mut buf = [0u8; 4];
        match self.encode_utf8(buf.as_mut_slice()) {
            Some(p) => Pattern::suffix_of(&p, s),
            None => None,
        }
    }
}

/// Are the bytes of `h` from `at` on those of `n`, all of which `h` has?
fn bytes_at(h: &[u8], n: &[u8], at: { a: usize | a + n.len() <= h.len() }) -> bool {
    let mut j = 0usize;
    while j < n.len() {
        if h[at + j] != n[j] {
            return false;
        }
        j = j + 1;
    }
    true
}

/// What `split` gives: the pieces of `rest` between where `pat` is found,
/// `empty` if what was found last was empty, at the start of `rest`, and
/// `done` once the last piece is given.
#[derive(Copy where P: Copy, Discard where P: Discard, Send where P: Send, Sync where P: Sync)]
export struct Split<'a, P> { rest: &'a str, pat: P, empty: bool, done: bool }

impl<'a, P: Pattern> Iterator for Split<'a, P> {
    type Item = &'a str;
    effect e = {};

    fn next(&mut self) -> Option<&'a str> {
        if self.done {
            return None;
        }
        let rest = self.rest;
        // Past an empty string found at the start of `rest`, the next is
        // looked for after its first character, if it has one.
        let from = match self.empty {
            true => match decode(rest.as_bytes(), 0) {
                Some(d) => Some(d.next),
                None => None,
            },
            false => Some(0),
        };
        let found = match from {
            Some(from) => self.pat.find_in(rest, from),
            None => None,
        };
        match found {
            Some(f) => {
                self.rest = after(rest, f.end);
                self.empty = f.start == f.end;
                Some(before(rest, f.start))
            },
            None => {
                self.done = true;
                Some(rest)
            },
        }
    }
}

/// What `split_whitespace` gives: the words of `rest`.
#[derive(Copy, Discard, Send, Sync)]
export struct SplitWhitespace<'a> { rest: &'a str }

impl<'a> Iterator for SplitWhitespace<'a> {
    type Item = &'a str;
    effect e = {};

    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        let b = rest.as_bytes();
        let mut at = 0usize;
        loop {
            match decode(b, at) {
                Some(d) => match d.c.is_whitespace() {
                    false => at = d.next,
                    true => break,
                },
                None => break,
            }
        }
        self.rest = after(rest, at);
        Some(before(rest, at))
    }
}

/// What `lines` gives: the lines of `rest`.
#[derive(Copy, Discard, Send, Sync)]
export struct Lines<'a> { rest: &'a str }

impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;
    effect e = {};

    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest;
        if rest.is_empty() {
            return None;
        }
        let line = match '\n'.find_in(rest, 0) {
            Some(f) => {
                self.rest = after(rest, f.end);
                before(rest, f.start)
            },
            None => {
                self.rest = "";
                rest
            },
        };
        Some(line.strip_suffix('\r').unwrap_or(line))
    }
}

/// What can be read from a string: `s.parse::<T>()`, or `T::parse(s)`.
/// The integers, `bool` and `char` can, and other types by impls of their
/// own.
export trait Parse {
    /// What's wrong with a string that isn't one.
    type Error;

    /// `s`, read as one, or what's wrong with it.
    fn parse(s: &str) -> Result<Self, Self::Error>;
}

/// `true` or `false`, written so, and nothing else.
impl Parse for bool {
    type Error = ParseBoolError;

    fn parse(s: &str) -> Result<bool, ParseBoolError> {
        match s {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ParseBoolError {}),
        }
    }
}

/// What's wrong with a string that isn't a `bool`: it's neither `true`
/// nor `false`.
#[derive(Copy, Debug, Discard, Eq, Send, Sync)]
export struct ParseBoolError {}

impl Display for ParseBoolError {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str("provided string was not `true` or `false`")
    }
}

/// One character, and nothing else.
impl Parse for char {
    type Error = ParseCharError;

    fn parse(s: &str) -> Result<char, ParseCharError> {
        let mut cs = s.chars();
        match cs.next() {
            Some(c) => match cs.next() {
                Some(_) => Err(ParseCharError::TooMany),
                None => Ok(c),
            },
            None => Err(ParseCharError::Empty),
        }
    }
}

/// What's wrong with a string that isn't a `char`.
#[derive(Copy, Debug, Discard, Eq, Send, Sync)]
export enum ParseCharError {
    /// It has none.
    Empty,
    /// It has more than one.
    TooMany,
}

impl Display for ParseCharError {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        match self {
            ParseCharError::Empty => w.write_str("cannot parse char from empty string"),
            ParseCharError::TooMany => w.write_str("too many characters in string"),
        }
    }
}

/// Bytes `b`, as a string, if they're UTF-8, or `None`.
export fn from_utf8(b: &[u8]) -> Option<&str> {
    let n = b.len();
    let mut at = 0usize;
    while at < n {
        match decode(b, at) {
            Some(d) => at = d.next,
            None => return None,
        }
    }
    // SAFETY: every byte is in a character `decode` found.
    Some(unsafe { from_utf8_unchecked(b) })
}

/// Bytes `b`, as a string.
///
/// # Safety
///
/// They're UTF-8.
#[intrinsic]
export fn from_utf8_unchecked(b: &[u8]) -> { s: &str | s.len() == b.len() } with unsafe;

/// What `split_at` gives: the string before the index it's given, and
/// from it on.
#[derive(Copy, Discard, Send, Sync)]
export struct StrSplit<'a> { export head: &'a str, export tail: &'a str }

/// What `chars` gives: each character of `bytes` from `at` on.
#[derive(Discard, Send, Sync)]
export struct Chars<'a> { bytes: &'a [u8], at: usize }

/// Safe code over `decode`. A `str`'s bytes are UTF-8, so it stops only
/// at their end; were they not, it would stop at the first that isn't.
impl<'a> Iterator for Chars<'a> {
    type Item = char;
    effect e = {};

    fn next(&mut self) -> Option<char> {
        match decode(self.bytes, self.at) {
            Some(d) => {
                self.at = d.next;
                Some(d.c)
            },
            None => None,
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
        return None;
    }
    let x = b[at] as u32;
    if x < 0x80 {
        return Some(Decoded { c: x as char, next: at + 1 });
    }
    // A continuation byte can't come first, `C0` and `C1` could only start
    // an overlong two, and `F5` and up something past `10FFFF`.
    if x < 0xC2 || x > 0xF4 {
        return None;
    }
    if at >= n - 1 {
        return None;
    }
    let c1 = match continuation(b, at + 1) {
        Some(c) => c.bits,
        None => return None,
    };
    if x < 0xE0 {
        let v = (x - 0xC0) * 0x40 + c1;
        return Some(Decoded { c: v as char, next: at + 2 });
    }
    if at + 1 >= n - 1 {
        return None;
    }
    let c2 = match continuation(b, at + 2) {
        Some(c) => c.bits,
        None => return None,
    };
    if x < 0xF0 {
        let v = (x - 0xE0) * 0x1000 + c1 * 0x40 + c2;
        if v < 0x800 || (v >= 0xD800 && v <= 0xDFFF) {
            return None;
        }
        return Some(Decoded { c: v as char, next: at + 3 });
    }
    if at + 2 >= n - 1 {
        return None;
    }
    let c3 = match continuation(b, at + 3) {
        Some(c) => c.bits,
        None => return None,
    };
    let v = (x - 0xF0) * 0x40000 + c1 * 0x1000 + c2 * 0x40 + c3;
    if v < 0x10000 || v > 0x10FFFF {
        return None;
    }
    Some(Decoded { c: v as char, next: at + 4 })
}

/// The six bits of a continuation byte, `10xxxxxx`.
#[derive(Copy, Discard)]
struct Six { bits: { c: u32 | c <= 0x3F } }

/// The six bits of byte `i` of `b`, if it's a continuation byte.
fn continuation(b: &[u8], i: { i: usize | i < b.len() }) -> Option<Six> {
    let y = b[i];
    if y < 0x80 || y > 0xBF {
        return None;
    }
    Some(Six { bits: (y - 0x80) as u32 })
}
