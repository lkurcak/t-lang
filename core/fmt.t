//! Formatting: values written as text, into a sink.
//!
//! A `Write` is where text goes, and says how writing to it can fail, by
//! its `Error`, and what writing to it does, by its effect `e`. A `Display`
//! is a value that can be written as text: its `fmt` writes it into any
//! sink, doing only what the sink does, so writing a number into a sink
//! that can't fail or panic can't fail or panic either.
//!
//! A `Debug` is a value that can be written as text for whoever is
//! debugging the program: a string in quotes, with its escapes, a struct
//! with its name and fields. `#[derive(Debug)]` writes it for a struct or
//! enum, field by field.
//!
//! `f"x = {x}, p = {p:?}"` is a value that writes its text, with `x`'s
//! `Display` and `p`'s `Debug` in place of the holes: it's `Display`
//! itself, so it goes wherever one does, as in `out.line(&f"..")?`. It's
//! made of `cat`, `display` and `debug` below, and needs nothing in scope.
//!
//! None of it is in the prelude: a module that writes or implements them
//! says `use core::fmt::{Display, Write};`.

use crate::result::Result;
use crate::slice::Iter;

/// Where text goes: a string, or one day a file or the terminal. Writing
/// gives `Ok`, or an `Error` if the sink can't take it, and does what `e`
/// says. A sink that can't fail says `type Error = !;`, and then what it
/// gives can be dropped.
export trait Write {
    /// What writing gives when the sink can't take what's written.
    type Error;

    /// What writing may do.
    effect e;

    /// Writes all of `s`, or else gives an error.
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> with Self::e;

    /// Writes `c`, or else gives an error.
    fn write_char(&mut self, c: char) -> Result<(), Self::Error> with Self::e;

    /// Writes `x` as text: `x.fmt(self)`, with the sink first, so that
    /// `w.put(&x)?` reads in the order things go.
    #[blame_caller]
    fn put<D: Display>(&mut self, x: &D) -> Result<(), Self::Error> with Self::e { x.fmt(self) }

    /// Writes `x` as text, then a line break.
    #[blame_caller]
    fn line<D: Display>(&mut self, x: &D) -> Result<(), Self::Error> with Self::e {
        x.fmt(self)?;
        self.write_char('\n')
    }
}

/// Writing through a mutable reference writes to what it points to.
impl<'a, W: Write> Write for &'a mut W {
    type Error = W::Error;
    effect e = W::e;

    #[blame_caller]
    fn write_str(&mut self, s: &str) -> Result<(), W::Error> with W::e { (**self).write_str(s) }

    #[blame_caller]
    fn write_char(&mut self, c: char) -> Result<(), W::Error> with W::e { (**self).write_char(c) }
}

/// What can be written as text, for people to read: an integer in
/// decimal, a `bool` as `true` or `false`, a string as its characters.
export trait Display {
    /// Writes it into `w`. It does only what `w` does, and gives what `w`
    /// gives: so it can't panic, or allocate, unless `w` can.
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e;
}

/// `true` or `false`.
impl Display for bool {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        if *self { w.write_str("true") } else { w.write_str("false") }
    }
}

/// The character itself.
impl Display for char {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_char(*self) }
}

/// Its characters, as they are.
impl Display for str {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str(self) }
}

/// Its characters: as `str`'s, which a type parameter can't stand for,
/// so `&T`'s doesn't cover it.
impl<'a> Display for &'a str {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str(*self) }
}

/// What it points to.
impl<'a, T: Display> Display for &'a T {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { (**self).fmt(w) }
}

/// What it points to.
impl<'a, T: Display> Display for &'a mut T {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { (**self).fmt(w) }
}

/// In decimal, with a `-` if it's negative: one impl for all ten integer
/// types, which overlaps none of the others, as `Int` is sealed (see
/// docs/29-closed-and-sealed-traits.md).
impl<T: Int> Display for T {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// `n` in decimal, with a `-` if it's negative.
#[blame_caller]
fn decimal<T: Int, W: Write>(n: T, w: &mut W) -> Result<(), W::Error> with W::e {
    if n < 0 {
        w.write_char('-')?;
    }
    digits(n, w)
}

/// The digits of `m`, most significant first, without its sign.
///
/// It works on `m` as it is, negative or not, as the negation of the least
/// value of a signed type doesn't fit in it. Dividing rounds towards zero,
/// so `m / 10` has `m`'s digits but the last, and `m % 10`, which has `m`'s
/// sign, is the last, or its negation: the checker proves that's 0 to 9,
/// so that none of it can panic, of every integer type.
#[blame_caller]
fn digits<T: Int, W: Write>(m: T, w: &mut W) -> Result<(), W::Error> with W::e {
    let q = m / 10;
    if q != 0 {
        digits(q, w)?;
    }
    let r = m % 10;
    let d = if r < 0 { -r } else { r };
    w.write_char((d as u32 + 0x30) as char)
}

/// What can be written as text for whoever is debugging the program: a
/// string or character in quotes, with escapes for what can't be seen, a
/// struct with its name and fields, `Point { x: 1, y: -2 }`, a variant with
/// its name and fields, `Some(3)`, a slice or array with its values,
/// `[1, 2]`. `#[derive(Debug)]` writes it, field by field, for a struct or
/// enum whose fields are `Debug`; `f"{x:?}"` writes it.
///
/// Its method is called `fmt`, as `Display`'s is: where both are in
/// scope, and a type implements both, `Debug::fmt(&x, w)` says which.
export trait Debug {
    /// Writes it into `w`. It does only what `w` does, and gives what `w`
    /// gives, as `Display::fmt` does.
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e;
}

/// `true` or `false`.
impl Debug for bool {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { Display::fmt(self, w) }
}

/// In decimal, as its `Display`.
impl<T: Int> Debug for T {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In single quotes, escaped as in a character literal: `'a'`, `'\''`,
/// `'\n'`.
impl Debug for char {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_char('\'')?;
        escaped(*self, '\'', w)?;
        w.write_char('\'')
    }
}

/// In double quotes, escaped as in a string literal: `"say \"hi\"\n"`.
impl Debug for str {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { quoted(self, w) }
}

/// As `str`'s, which a type parameter can't stand for, so `&T`'s doesn't
/// cover it.
impl<'a> Debug for &'a str {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { quoted(*self, w) }
}

/// `()`.
impl Debug for () {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("()") }
}

/// What it points to.
impl<'a, T: Debug> Debug for &'a T {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { Debug::fmt(*self, w) }
}

/// What it points to.
impl<'a, T: Debug> Debug for &'a mut T {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { Debug::fmt(&**self, w) }
}

/// Its values, in brackets: `[1, 2, 3]`.
impl<T: Debug> Debug for [T] {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { items(self.iter(), w) }
}

/// Its values, as `[T]`'s, which a type parameter can't stand for, so
/// `&T`'s doesn't cover it.
impl<'a, T: Debug> Debug for &'a [T] {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { items(self.iter(), w) }
}

/// Its values, as `&[T]`'s.
impl<'a, T: Debug> Debug for &'a mut [T] {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { items(self.iter(), w) }
}

/// Its values, as a slice's.
impl<T: Debug, const N: usize> Debug for [T; N] {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        Debug::fmt(self.as_slice(), w)
    }
}

/// The values `xs` gives, in brackets, by their `Debug`: `[1, 2, 3]`.
#[blame_caller]
fn items<'a, T: Debug, W: Write>(xs: Iter<'a, T>, w: &mut W) -> Result<(), W::Error> with W::e {
    w.write_char('[')?;
    let mut first = true;
    for x in xs {
        if !first {
            w.write_str(", ")?;
        }
        first = false;
        Debug::fmt(x, w)?;
    }
    w.write_char(']')
}

/// `s` in double quotes, escaped.
#[blame_caller]
fn quoted<W: Write>(s: &str, w: &mut W) -> Result<(), W::Error> with W::e {
    w.write_char('"')?;
    for c in s.chars() {
        escaped(c, '"', w)?;
    }
    w.write_char('"')
}

/// `c` as it is between `quote`s in a literal: the escape of a quote,
/// backslash, tab, line break, carriage return or nul, `\u{..}` for any
/// other control character of ASCII, and else itself.
#[blame_caller]
fn escaped<W: Write>(c: char, quote: char, w: &mut W) -> Result<(), W::Error> with W::e {
    match c {
        '\t' => w.write_str("\\t"),
        '\r' => w.write_str("\\r"),
        '\n' => w.write_str("\\n"),
        '\\' => w.write_str("\\\\"),
        '\0' => w.write_str("\\0"),
        '\u{1}'..='\u{1F}' | '\u{7F}' => {
            w.write_str("\\u{")?;
            hex(c as u32, w)?;
            w.write_char('}')
        },
        c => {
            if c == quote {
                w.write_char('\\')?;
            }
            w.write_char(c)
        },
    }
}

/// The digits of `n` in lowercase hexadecimal, most significant first.
#[blame_caller]
fn hex<W: Write>(n: u32, w: &mut W) -> Result<(), W::Error> with W::e {
    let q = n / 16;
    if q != 0 {
        hex(q, w)?;
    }
    let d = n % 16;
    let c = if d < 10 { d + 0x30 } else { d + 0x57 };
    w.write_char(c as char)
}

/// Two values written one after the other: what `f"a{x}"` is made of,
/// with `cat` (see the module's doc).
#[derive(Copy where A: Copy, B: Copy, Discard where A: Discard, B: Discard)]
export struct Cat<A, B> { a: A, b: B }

/// `a`, then `b`: a piece of an `f".."`, then the rest of it.
export fn cat<A: Display, B: Display>(a: A, b: B) -> Cat<A, B> { Cat { a, b } }

/// `a`'s text, then `b`'s.
impl<A: Display, B: Display> Display for Cat<A, B> {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        self.a.fmt(w)?;
        self.b.fmt(w)
    }
}

/// `x` as itself: a `{x}` of an `f".."`, which must be `Display`.
export fn display<'a, T: Display>(x: &'a T) -> &'a T { x }

/// A value written by its `Debug`: a `{x:?}` of an `f".."`, made with
/// `debug`.
#[derive(Copy, Discard)]
export struct Dbg<'a, T> { x: &'a T }

/// `x`, to be written by its `Debug`.
export fn debug<'a, T: Debug>(x: &'a T) -> Dbg<'a, T> { Dbg { x } }

/// `x`'s `Debug`.
impl<'a, T: Debug> Display for Dbg<'a, T> {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { Debug::fmt(self.x, w) }
}
