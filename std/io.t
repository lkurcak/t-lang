//! Input and output: the operating system's streams. Each is a value no
//! code can make, which `main` is given in its `Os`: a function can read
//! or write one only if it was handed it, or a reference to it.
//!
//! `Stdin` is a `core::io::Read`, and `Stdout` and `Stderr` are each a
//! `core::io::Write` of bytes and a `core::fmt::Write` of text: so
//! `out.put(&x)?` writes `x`, and `out.line(&x)?` writes it and a line
//! break. Each can fail, as a stream may be closed or a disk full, with an
//! `Error`, the one `core::io` has.

use core::fmt;
use core::fmt::Display;
use core::io::Read;
use core::num::wrap;

use alloc::list::List;
use alloc::string::String;

export use core::io::{Error, ErrorKind};

/// Where a program's input comes from.
#[derive(Discard, Send, Sync)]
#[intrinsic]
export struct Stdin {}

/// Where a program's output goes.
#[derive(Discard, Send, Sync)]
#[intrinsic]
export struct Stdout {}

/// Where a program says what went wrong.
#[derive(Discard, Send, Sync)]
#[intrinsic]
export struct Stderr {}

impl Stdin {
    /// The next line of input, without its line break (`\n`, or `\r\n`),
    /// in `line`, instead of what it held: `alloc::io::read_line`, without
    /// needing it in scope. `Ok(false)` if there's no more input.
    #[blame_caller]
    export fn read_line(&mut self, line: &mut String) -> Result<bool, Error> with alloc {
        alloc::io::read_line(self, line)
    }

    /// Reads all the input that's left onto the end of `buf`, and gives
    /// how many bytes that was: `alloc::io::read_to_end`.
    #[blame_caller]
    export fn read_to_end(&mut self, buf: &mut List<u8>) -> Result<usize, Error> with alloc {
        alloc::io::read_to_end(self, buf)
    }

    /// Reads all the input that's left onto the end of `s`, which must be
    /// UTF-8: `alloc::io::read_to_string`.
    #[blame_caller]
    export fn read_to_string(&mut self, s: &mut String) -> Result<usize, Error> with alloc {
        alloc::io::read_to_string(self, s)
    }

    /// Is the input a terminal, that someone types into, rather than a
    /// file or a pipe?
    export fn is_terminal(&self) -> bool { self.os_is_terminal() }

    /// Reads into `buf`, as `Read::read`: how many bytes, or an error, as
    /// `status` says.
    #[intrinsic]
    fn os_read(&mut self, buf: &mut [u8]) -> u64;

    #[intrinsic]
    fn os_is_terminal(&self) -> bool;
}

/// The program's input. It's read through a buffer, so a byte at a time
/// is quick, and a read gives what one read of the operating system's
/// gave, without waiting for more.
impl Read for Stdin {
    effect e = {};

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let r = self.os_read(buf);
        match failure(r) {
            Some(e) => Err(e),
            None => Ok(wrap(r)),
        }
    }
}

impl Stdout {
    /// Writes `x` as text: what `Write::put` does, without needing `Write`
    /// in scope.
    export fn put<D: Display>(&mut self, x: &D) -> Result<(), Error> { x.fmt(self) }

    /// Writes `x` as text, then a line break.
    export fn line<D: Display>(&mut self, x: &D) -> Result<(), Error> {
        x.fmt(self)?;
        fmt::Write::write_char(self, '\n')
    }

    /// Writes out what's waiting in the buffer: the output is written
    /// when the buffer fills, before input is read, and when `main`
    /// returns, and otherwise when this says.
    export fn flush(&mut self) -> Result<(), Error> { status(self.os_flush()) }

    /// Is the output a terminal, that someone reads, rather than a file
    /// or a pipe?
    export fn is_terminal(&self) -> bool { self.os_is_terminal() }

    /// Writes all of `buf`: 0, or an error, as `status` says.
    #[intrinsic]
    fn os_write(&mut self, buf: &[u8]) -> u64;

    #[intrinsic]
    fn os_flush(&mut self) -> u64;

    #[intrinsic]
    fn os_is_terminal(&self) -> bool;
}

impl Stderr {
    /// Writes `x` as text: what `Write::put` does, without needing `Write`
    /// in scope.
    export fn put<D: Display>(&mut self, x: &D) -> Result<(), Error> { x.fmt(self) }

    /// Writes `x` as text, then a line break.
    export fn line<D: Display>(&mut self, x: &D) -> Result<(), Error> {
        x.fmt(self)?;
        fmt::Write::write_char(self, '\n')
    }

    /// Nothing waits: what's written to where errors go is written at once.
    export fn flush(&mut self) -> Result<(), Error> { status(self.os_flush()) }

    /// Is where errors go a terminal, rather than a file or a pipe?
    export fn is_terminal(&self) -> bool { self.os_is_terminal() }

    /// Writes all of `buf`: 0, or an error, as `status` says.
    #[intrinsic]
    fn os_write(&mut self, buf: &[u8]) -> u64;

    #[intrinsic]
    fn os_flush(&mut self) -> u64;

    #[intrinsic]
    fn os_is_terminal(&self) -> bool;
}

/// Bytes written to the program's output.
impl core::io::Write for Stdout {
    effect e = {};

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> { status(self.os_write(buf)) }

    fn flush(&mut self) -> Result<(), Error> { status(self.os_flush()) }
}

/// Bytes written to where errors go.
impl core::io::Write for Stderr {
    effect e = {};

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> { status(self.os_write(buf)) }

    fn flush(&mut self) -> Result<(), Error> { status(self.os_flush()) }
}

/// Text written to the program's output, in UTF-8.
impl fmt::Write for Stdout {
    type Error = Error;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), Error> { status(self.os_write(s.as_bytes())) }

    fn write_char(&mut self, c: char) -> Result<(), Error> {
        let mut b = [0u8; 4];
        match c.encode_utf8(b.as_mut_slice()) {
            Some(s) => status(self.os_write(s.as_bytes())),
            None => Err(Error::new(ErrorKind::Other)),
        }
    }
}

/// Text written to where errors go, in UTF-8.
impl fmt::Write for Stderr {
    type Error = Error;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), Error> { status(self.os_write(s.as_bytes())) }

    fn write_char(&mut self, c: char) -> Result<(), Error> {
        let mut b = [0u8; 4];
        match c.encode_utf8(b.as_mut_slice()) {
            Some(s) => status(self.os_write(s.as_bytes())),
            None => Err(Error::new(ErrorKind::Other)),
        }
    }
}

/// What a stream's intrinsic gives back, `r`, as a result: below bit 63,
/// it worked, and that's how many bytes; with bit 63, it failed, with the
/// operating system's number for why in the low 32 bits, and which
/// `ErrorKind` that is, by its place in the enum, in the 8 above them.
pub fn failure(r: u64) -> Option<Error> {
    if r >> 63 == 0 {
        return None;
    }
    let kind = match (r >> 32) & 0xFF {
        0 => ErrorKind::NotFound,
        1 => ErrorKind::PermissionDenied,
        2 => ErrorKind::AlreadyExists,
        3 => ErrorKind::BrokenPipe,
        4 => ErrorKind::InvalidData,
        5 => ErrorKind::UnexpectedEnd,
        6 => ErrorKind::Unsupported,
        7 => ErrorKind::InvalidInput,
        _ => ErrorKind::Other,
    };
    Some(Error::os(kind, wrap(r & 0x7FFFFFFF)))
}

/// The error `r` says, as `failure` reads it, from an intrinsic that
/// gives back only failures.
pub fn error(r: u64) -> Error {
    match failure(r) {
        Some(e) => e,
        None => Error::new(ErrorKind::Other),
    }
}

/// `Ok`, or the error `r` says, as `failure` reads it.
pub fn status(r: u64) -> Result<(), Error> {
    match failure(r) {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
