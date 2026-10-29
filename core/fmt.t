//! Formatting: values written as text, into a sink.
//!
//! A `Write` is where text goes, and says how writing to it can fail, by
//! its `Error`, and what writing to it does, by its effect `e`. A `Display`
//! is a value that can be written as text: its `fmt` writes it into any
//! sink, doing only what the sink does, so writing a number into a sink
//! that can't fail or panic can't fail or panic either.
//!
//! Neither is in the prelude: a module that writes says
//! `use core::fmt::{Display, Write};`.

use crate::result::Result;

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
    fn put<D: Display>(&mut self, x: &D) -> Result<(), Self::Error> with Self::e { x.fmt(self) }

    /// Writes `x` as text, then a line break.
    fn line<D: Display>(&mut self, x: &D) -> Result<(), Self::Error> with Self::e {
        x.fmt(self)?;
        self.write_char('\n')
    }
}

/// Writing through a mutable reference writes to what it points to.
impl<'a, W: Write> Write for &'a mut W {
    type Error = W::Error;
    effect e = W::e;

    fn write_str(&mut self, s: &str) -> Result<(), W::Error> with W::e { (**self).write_str(s) }

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
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        if *self { w.write_str("true") } else { w.write_str("false") }
    }
}

/// The character itself.
impl Display for char {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_char(*self) }
}

/// Its characters, as they are.
impl Display for str {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str(self) }
}

/// Its characters: as `str`'s, which a type parameter can't stand for,
/// so `&T`'s doesn't cover it.
impl<'a> Display for &'a str {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str(*self) }
}

/// What it points to.
impl<'a, T: Display> Display for &'a T {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { (**self).fmt(w) }
}

/// What it points to.
impl<'a, T: Display> Display for &'a mut T {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { (**self).fmt(w) }
}

/// In decimal, with a `-` if it's negative.
impl Display for i8 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal, with a `-` if it's negative.
impl Display for i16 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal, with a `-` if it's negative.
impl Display for i32 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal, with a `-` if it's negative.
impl Display for i64 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal, with a `-` if it's negative.
impl Display for isize {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal.
impl Display for u8 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal.
impl Display for u16 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal.
impl Display for u32 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal.
impl Display for u64 {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// In decimal.
impl Display for usize {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { decimal(*self, w) }
}

/// `n` in decimal, with a `-` if it's negative: once for every integer
/// type, which the ten impls above call, as one impl for every `T: Int`
/// would overlap those for `bool` and `char`.
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
fn digits<T: Int, W: Write>(m: T, w: &mut W) -> Result<(), W::Error> with W::e {
    let q = m / 10;
    if q != 0 {
        digits(q, w)?;
    }
    let r = m % 10;
    let d = if r < 0 { -r } else { r };
    w.write_char((d as u32 + 0x30) as char)
}
