// Strings that grow: text in UTF-8, owned, on the heap.

use core::cmp::{Ord, Ordering};
use core::str;

use crate::list::List;

/// A string of its own: bytes that are UTF-8, in a list on the heap, which
/// grows as characters and strings are pushed. `as_str` sees them as a
/// `&str`.
///
/// It isn't `Discard`, as its list isn't: it's used up by `free`, or by
/// `into_bytes`.
///
/// # Invariant
///
/// The bytes of `bytes` are UTF-8. The one `unsafe` block here, in
/// `as_str`, counts on it, and every method keeps it, though they're safe
/// code: so the block is right only if they are. Its field is private, so
/// nothing else can break it.
pub struct String { bytes: List<u8> }

impl String {
    /// An empty string. It allocates nothing until something's pushed.
    pub fn new() -> String { String { bytes: List::new() } }

    /// An empty string with room for `cap` bytes before it grows.
    pub fn with_capacity(cap: usize) -> String with {alloc, panic} {
        String { bytes: List::with_capacity(cap) }
    }

    /// A string of the characters of `s`.
    pub fn from(s: &str) -> String with {alloc, panic} {
        let mut out = String::with_capacity(s.len());
        out.push_str(s);
        out
    }

    /// Bytes `b`, as a string, if they're UTF-8, or else `b` back.
    pub fn from_utf8(b: List<u8>) -> Result<String, List<u8>> {
        if str::from_utf8(b.as_slice()).is_some() {
            Result::Ok(String { bytes: b })
        } else {
            Result::Err(b)
        }
    }

    /// Its bytes, as a list, which uses it up.
    pub fn into_bytes(self) -> List<u8> {
        let String { bytes } = self;
        bytes
    }

    /// Its characters, as a `str`.
    pub fn as_str(&self) -> &str {
        // SAFETY: its bytes are UTF-8, by the invariant.
        unsafe { str::from_utf8_unchecked(self.bytes.as_slice()) }
    }

    /// Its bytes, its UTF-8.
    pub fn as_bytes(&self) -> &[u8] { self.bytes.as_slice() }

    /// How many bytes it has: not how many characters.
    pub fn len(&self) -> usize { self.bytes.len() }

    /// Has it none?
    pub fn is_empty(&self) -> bool { self.bytes.is_empty() }

    /// How many bytes it has room for before it grows.
    pub fn capacity(&self) -> usize { self.bytes.capacity() }

    /// Puts `c` at the end.
    ///
    /// It pushes a zero byte for each byte `c` takes, and then writes `c`
    /// over them: zeros are UTF-8, so the invariant holds all along.
    pub fn push(&mut self, c: char) with {alloc, panic} {
        let at = self.bytes.len();
        let n = c.len_utf8();
        let mut i = 0usize;
        while i < n {
            self.bytes.push(0);
            i = i + 1;
        }
        match self.bytes.as_mut_slice().split_at_mut(at) {
            Option::Some(split) => {
                c.encode_utf8(split.tail);
            },
            Option::None => {},
        }
    }

    /// Puts the characters of `s` at the end. Its bytes are UTF-8, so
    /// they're UTF-8 after the string's.
    pub fn push_str(&mut self, s: &str) with {alloc, panic} {
        for b in s.as_bytes().iter() {
            self.bytes.push(*b);
        }
    }

    /// Takes the last character out, or `None` if it's empty.
    pub fn pop(&mut self) -> Option<char> {
        let n = self.bytes.len();
        if n == 0 {
            return Option::None;
        }
        let mut at = n - 1;
        while at > 0 && !self.as_str().is_char_boundary(at) {
            at = at - 1;
        }
        let c = match self.as_str().get(at, n) {
            Option::Some(last) => last.chars().next(),
            Option::None => Option::None,
        };
        // Cut where the last character starts, so the rest is UTF-8.
        self.bytes.truncate(at);
        c
    }

    /// Throws away the bytes from index `n` on, if there are any, so it
    /// holds at most `n`, and says `true`; or, if no character starts at
    /// `n`, leaves it as it is, and says `false`.
    pub fn truncate(&mut self, n: usize) -> bool {
        if n >= self.bytes.len() {
            return true;
        }
        if !self.as_str().is_char_boundary(n) {
            return false;
        }
        // Cut where a character starts, so the rest is UTF-8.
        self.bytes.truncate(n);
        true
    }

    /// Throws its characters away, keeping the buffer for more.
    pub fn clear(&mut self) { self.bytes.clear() }

    /// Gives its buffer back.
    pub fn free(self) { self.bytes.free() }
}

/// The same characters.
impl Eq for String {
    fn eq(&self, other: &String) -> bool { self.as_str() == other.as_str() }
}

/// As their `str`s order.
impl Ord for String {
    fn cmp(&self, other: &String) -> Ordering { self.as_str().cmp(other.as_str()) }
}
