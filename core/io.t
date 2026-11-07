//! Input and output of bytes: where they come from, a `Read`, and where
//! they go, a `Write`. A file, the terminal, or bytes in memory, all the
//! same to code generic over them (see docs/47-io.md).
//!
//! Each says what reading or writing does by its effect `e`, and fails
//! with the one `Error` there is, whatever the stream: so `?` passes on a
//! failure to read input and one to write output alike. Text goes into a
//! `Write` as into any `core::fmt::Write`: `w.put(&x)?`.
//!
//! None of it is in the prelude: a module says `use core::io::{Read, Write};`.

use crate::fmt;
use crate::fmt::Display;
use crate::option::Option;
use crate::result::Result;
use crate::slice::{Split, SplitMut};

/// What kind of failure an `Error` is.
#[derive(Discard, Copy, Debug, Eq, Send, Sync)]
export enum ErrorKind {
    /// What was asked for isn't there: a file, say.
    NotFound,
    /// The operating system doesn't let the program do it.
    PermissionDenied,
    /// What was to be made is there already.
    AlreadyExists,
    /// What's at the other end went away: a pipe's reader, say.
    BrokenPipe,
    /// What was read isn't what was asked for: text that isn't UTF-8.
    InvalidData,
    /// The input ended before what was asked for was read.
    UnexpectedEnd,
    /// It can't be done here: on this operating system, or this stream.
    Unsupported,
    /// Something else.
    Other,
}

/// Why reading or writing failed: its kind, and the operating system's
/// own number for it, if the operating system said so.
#[derive(Discard, Copy, Debug, Eq, Send, Sync)]
export struct Error {
    kind: ErrorKind,
    /// The operating system's number for it (`errno`), or 0 if it didn't
    /// come from the operating system: no failure there is 0.
    code: i32,
}

impl Error {
    /// An error of kind `kind`.
    export fn new(kind: ErrorKind) -> Error { Error { kind, code: 0 } }

    /// An error of kind `kind` that the operating system gave as `code`.
    export fn os(kind: ErrorKind, code: i32) -> Error { Error { kind, code } }

    /// What kind it is.
    export fn kind(&self) -> ErrorKind { self.kind }

    /// The operating system's number for it, if it came from there.
    export fn os_code(&self) -> Option<i32> {
        if self.code == 0 { Option::None } else { Option::Some(self.code) }
    }
}

/// What went wrong, for people to read, and the operating system's
/// number for it, if it has one: `broken pipe (os error 32)`.
impl Display for Error {
    #[blame_caller]
    fn fmt<W: fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str(match self.kind {
            ErrorKind::NotFound => "not found",
            ErrorKind::PermissionDenied => "permission denied",
            ErrorKind::AlreadyExists => "already exists",
            ErrorKind::BrokenPipe => "broken pipe",
            ErrorKind::InvalidData => "stream did not contain valid UTF-8",
            ErrorKind::UnexpectedEnd => "unexpected end of input",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::Other => "input/output error",
        })?;
        if self.code != 0 {
            w.write_str(" (os error ")?;
            self.code.fmt(w)?;
            w.write_char(')')?;
        }
        Result::Ok(())
    }
}

/// Where bytes come from: the input, a file, or bytes in memory.
export trait Read {
    /// What reading may do.
    effect e;

    /// Reads some bytes into the front of `buf`, and gives how many: at
    /// most `buf.len()`, and at least one unless `buf` is empty or the
    /// input has ended. So `Ok(0)` for a `buf` that isn't empty says
    /// there's no more. It gives what's there, without waiting for more
    /// to fill `buf`.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> with Self::e;

    /// Reads exactly enough bytes to fill `buf`, or else fails: with
    /// `UnexpectedEnd` if the input ends first. What was read before a
    /// failure is in `buf`, but not how much.
    #[blame_caller]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> with Self::e {
        let mut rest = buf;
        while !rest.is_empty() {
            let n = self.read(rest)?;
            if n == 0 {
                return Result::Err(Error::new(ErrorKind::UnexpectedEnd));
            }
            let all = rest;
            match all.split_at_mut(n) {
                Option::Some(SplitMut { tail, .. }) => rest = tail,
                // `read` broke its promise.
                Option::None => return Result::Err(Error::new(ErrorKind::Other)),
            }
        }
        Result::Ok(())
    }
}

/// Reading through a mutable reference reads from what it points to.
impl<'a, R: Read> Read for &'a mut R {
    effect e = R::e;

    #[blame_caller]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> with R::e { (**self).read(buf) }
}

/// Bytes in memory to read: reading takes them from the front, so what's
/// left is what's not been read yet. It never fails.
#[derive(Discard, Copy, Debug, Send, Sync)]
export struct Bytes<'a> { rest: &'a [u8] }

impl<'a> Bytes<'a> {
    /// Reads `b`, from its start.
    export fn new(b: &'a [u8]) -> Bytes<'a> { Bytes { rest: b } }

    /// What's not been read yet.
    export fn rest(&self) -> &'a [u8] { self.rest }
}

impl<'a> Read for Bytes<'a> {
    effect e = {};

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let all = self.rest;
        let n = if buf.len() < all.len() { buf.len() } else { all.len() };
        let mut i = 0usize;
        while i < n {
            buf[i] = all[i];
            i = i + 1;
        }
        if let Option::Some(Split { tail, .. }) = all.split_at(n) {
            self.rest = tail;
        }
        Result::Ok(n)
    }
}

/// Where bytes go: the output, a file, or a list of bytes in memory.
/// What's written may wait in a buffer until `flush`. Text goes in with
/// `put` and `line`, in UTF-8.
export trait Write {
    /// What writing may do.
    effect e;

    /// Writes all of `buf`, or else fails. If it fails, some of it may
    /// have been written.
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> with Self::e;

    /// Writes out what's waiting in a buffer, if anything is, to where
    /// it's going.
    fn flush(&mut self) -> Result<(), Error> with Self::e;

    /// Writes `x` as text, in UTF-8: as `core::fmt::Write::put` does.
    #[blame_caller]
    fn put<D: Display>(&mut self, x: &D) -> Result<(), Error> with Self::e {
        x.fmt(&mut Text { sink: self })
    }

    /// Writes `x` as text, then a line break.
    #[blame_caller]
    fn line<D: Display>(&mut self, x: &D) -> Result<(), Error> with Self::e {
        x.fmt(&mut Text { sink: self })?;
        self.write_all("\n".as_bytes())
    }
}

/// Writing through a mutable reference writes to what it points to.
impl<'a, W: Write> Write for &'a mut W {
    effect e = W::e;

    #[blame_caller]
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> with W::e { (**self).write_all(buf) }

    #[blame_caller]
    fn flush(&mut self) -> Result<(), Error> with W::e { (**self).flush() }
}

/// A `Write` of bytes as a `core::fmt::Write` of text: what `put` and
/// `line` write through. A blanket `impl<W: Write> fmt::Write for W`
/// would overlap `String`'s, so it's this instead.
#[derive(Discard)]
struct Text<'a, W> { sink: &'a mut W }

/// Text, in UTF-8.
impl<'a, W: Write> fmt::Write for Text<'a, W> {
    type Error = Error;
    effect e = W::e;

    #[blame_caller]
    fn write_str(&mut self, s: &str) -> Result<(), Error> with W::e {
        self.sink.write_all(s.as_bytes())
    }

    #[blame_caller]
    fn write_char(&mut self, c: char) -> Result<(), Error> with W::e {
        let mut bytes = [0u8; 4];
        match c.encode_utf8(bytes.as_mut_slice()) {
            Option::Some(s) => self.sink.write_all(s.as_bytes()),
            Option::None => Result::Err(Error::new(ErrorKind::Other)),
        }
    }
}
