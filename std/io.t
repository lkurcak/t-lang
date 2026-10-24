//! Input and output: the operating system's streams. Each is a value no
//! code can make, which `main` is given in its `Os`: a function can read
//! or write one only if it was handed it, or a reference to it.
//!
//! Writing to one is writing text into a sink, as `core::fmt` says: so
//! `out.put(&x)?` writes `x`, and `out.line(&x)?` writes it and a line
//! break. Each can fail, as a stream may be closed or a disk full, with an
//! `Error`.

use core::fmt::{Display, Write};

use alloc::list::List;
use alloc::string::String;

/// Where a program's input comes from.
#[derive(Discard)]
#[intrinsic]
pub struct Stdin {}

/// Where a program's output goes.
#[derive(Discard)]
#[intrinsic]
pub struct Stdout {}

/// Where a program says what went wrong.
#[derive(Discard)]
#[intrinsic]
pub struct Stderr {}

/// What kind of failure an `Error` is.
#[derive(Discard, Copy, Eq)]
pub enum ErrorKind {
    /// What's at the other end went away: a pipe's reader, say.
    BrokenPipe,
    /// What was read isn't what was asked for: text that isn't UTF-8.
    InvalidData,
    /// Something else.
    Other,
}

/// Why reading or writing failed.
#[derive(Discard)]
pub struct Error { kind: ErrorKind }

impl Error {
    /// An error of kind `kind`.
    pub fn new(kind: ErrorKind) -> Error { Error { kind } }

    /// What kind it is.
    pub fn kind(&self) -> ErrorKind { self.kind }
}

/// What went wrong, for people to read.
impl Display for Error {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        match self.kind {
            ErrorKind::BrokenPipe => w.write_str("broken pipe"),
            ErrorKind::InvalidData => w.write_str("stream did not contain valid UTF-8"),
            ErrorKind::Other => w.write_str("input/output error"),
        }
    }
}

/// The error a stream's intrinsic gives back as `code`: one past a byte's,
/// so 257 and on. 257 is a broken pipe.
fn error(code: u32) -> Error {
    if code == 257 { Error::new(ErrorKind::BrokenPipe) } else { Error::new(ErrorKind::Other) }
}

impl Stdin {
    /// The next line of input, without its line break (`\n`, or `\r\n`),
    /// in `line`, instead of what it held. `Ok(false)` if there's no more
    /// input. The last line needn't end in a line break.
    ///
    /// Input that isn't UTF-8 is an error of kind `InvalidData`; the line
    /// is read, and `line` is left empty.
    pub fn read_line(&mut self, line: &mut String) -> Result<bool, Error> with alloc {
        line.clear();
        let mut bytes = List::new();
        let mut any = false;
        loop {
            let b = self.read_byte();
            if b < 256 {
                any = true;
                if b == 0x0A {
                    break;
                }
                bytes.push(b as u8);
            } else if b == 256 {
                break;
            } else {
                return Result::Err(error(b));
            }
        }
        match bytes.pop() {
            Option::Some(0x0D) => {},
            Option::Some(b) => bytes.push(b),
            Option::None => {},
        }
        match String::from_utf8(bytes) {
            Result::Ok(s) => {
                line.push_str(s.as_str());
                Result::Ok(any)
            }
            Result::Err(_) => Result::Err(Error::new(ErrorKind::InvalidData)),
        }
    }

    /// The next byte of input: below 256, a byte; 256, the end of the
    /// input; above, an error, as `error` says.
    #[intrinsic]
    fn read_byte(&mut self) -> u32;
}

impl Stdout {
    /// Writes `x` as text: what `Write::put` does, without needing `Write`
    /// in scope.
    pub fn put<D: Display>(&mut self, x: &D) -> Result<(), Error> { x.fmt(self) }

    /// Writes `x` as text, then a line break.
    pub fn line<D: Display>(&mut self, x: &D) -> Result<(), Error> {
        x.fmt(self)?;
        self.write_char('\n')
    }

    /// Writes byte `b`: 0 if it did, or else an error, as `error` says.
    #[intrinsic]
    fn write_byte(&mut self, b: u8) -> u32;
}

impl Stderr {
    /// Writes `x` as text: what `Write::put` does, without needing `Write`
    /// in scope.
    pub fn put<D: Display>(&mut self, x: &D) -> Result<(), Error> { x.fmt(self) }

    /// Writes `x` as text, then a line break.
    pub fn line<D: Display>(&mut self, x: &D) -> Result<(), Error> {
        x.fmt(self)?;
        self.write_char('\n')
    }

    /// Writes byte `b`: 0 if it did, or else an error, as `error` says.
    #[intrinsic]
    fn write_byte(&mut self, b: u8) -> u32;
}

/// Text written to the program's output.
impl Write for Stdout {
    type Error = Error;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), Error> { write_str(self, s) }

    fn write_char(&mut self, c: char) -> Result<(), Error> { write_char(self, c) }
}

/// Text written to where errors go.
impl Write for Stderr {
    type Error = Error;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), Error> { write_str(self, s) }

    fn write_char(&mut self, c: char) -> Result<(), Error> { write_char(self, c) }
}

/// A stream that takes bytes: what `Write` for `Stdout` and `Stderr` is
/// built on.
trait Bytes {
    /// Writes byte `b`, or else gives why it couldn't.
    fn byte(&mut self, b: u8) -> Result<(), Error>;
}

impl Bytes for Stdout {
    fn byte(&mut self, b: u8) -> Result<(), Error> {
        let code = self.write_byte(b);
        if code == 0 { Result::Ok(()) } else { Result::Err(error(code)) }
    }
}

impl Bytes for Stderr {
    fn byte(&mut self, b: u8) -> Result<(), Error> {
        let code = self.write_byte(b);
        if code == 0 { Result::Ok(()) } else { Result::Err(error(code)) }
    }
}

/// Writes the bytes of `s`, UTF-8 as a `str`'s are, one by one.
fn write_str<B: Bytes>(out: &mut B, s: &str) -> Result<(), Error> {
    for b in s.as_bytes().iter() {
        out.byte(*b)?;
    }
    Result::Ok(())
}

/// Writes `c`, in UTF-8.
fn write_char<B: Bytes>(out: &mut B, c: char) -> Result<(), Error> {
    let n = c as u32;
    if n < 0x80 {
        out.byte(n as u8)
    } else if n < 0x800 {
        out.byte(((n >> 6) & 0x1F) as u8 | 0xC0)?;
        out.byte(tail(n, 0))
    } else if n < 0x10000 {
        out.byte(((n >> 12) & 0x0F) as u8 | 0xE0)?;
        out.byte(tail(n, 6))?;
        out.byte(tail(n, 0))
    } else {
        out.byte(((n >> 18) & 0x07) as u8 | 0xF0)?;
        out.byte(tail(n, 12))?;
        out.byte(tail(n, 6))?;
        out.byte(tail(n, 0))
    }
}

/// A byte after the first in UTF-8: `10`, then six bits of `n`, from bit
/// `shift` up.
fn tail(n: u32, shift: u32) -> u8 { ((n >> shift) & 0x3F) as u8 | 0x80 }
