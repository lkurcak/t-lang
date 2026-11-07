//! Files: reading them, and writing them whole (see docs/48-files.md).
//!
//! The file system is reached through the `Fs` in the `Os` that `main` is
//! given, as `os.fs`: a function can read or write files only if it was
//! handed it, or a reference to it.
//!
//! A file is written whole, by one call that gives back one `Result`:
//! `fs.write(path, bytes)`, or `fs.append`. Build what's written first, in
//! a `String` or a `List<u8>`, which are each a `Write`. A file is read
//! whole the same way, with `fs.read` or `fs.read_to_string`, or bit by bit,
//! through a `ReadFile` that `fs.open` gives, which is a `Read` as `Stdin`
//! is.
//!
//! A path is text, `&str`: relative to the directory the program was
//! started in, or else from the root, `/`.

use core::io::Read;

use alloc::list::List;
use alloc::string::String;

use crate::io::{Error, ErrorKind};

/// The right to read and write files: what `fn main(os: Os)` is given in
/// its `os.fs`. It's `Sync`, so a `&Fs` may go to a thread.
#[derive(Discard, Send, Sync)]
#[intrinsic]
export struct Fs {}

/// A file opened to be read, by `Fs::open`. It's read through a buffer, as
/// `Stdin` is, so a byte at a time is quick. Dropping it closes it: it's
/// only read, so nothing is lost.
#[derive(Send, Sync)]
#[intrinsic]
export struct ReadFile {}

impl Fs {
    /// Opens the file at `path` to be read: `NotFound` if there's none,
    /// `PermissionDenied` if it may not be read.
    ///
    /// It's `with alloc`, as its buffer is memory: if there's none, the
    /// program stops.
    export fn open(&self, path: &str) -> Result<ReadFile, Error> with alloc {
        checked(path)?;
        match self.os_open(path) {
            Ok(f) => Ok(f),
            Err(r) => Err(crate::io::error(r)),
        }
    }

    /// All of the file at `path`, as bytes.
    #[blame_caller]
    export fn read(&self, path: &str) -> Result<List<u8>, Error> with alloc {
        let mut f = self.open(path)?;
        let mut bytes = List::new();
        let _ = f.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// All of the file at `path`, as text: `InvalidData` if it isn't UTF-8.
    #[blame_caller]
    export fn read_to_string(&self, path: &str) -> Result<String, Error> with alloc {
        let mut f = self.open(path)?;
        let mut s = String::new();
        let _ = f.read_to_string(&mut s)?;
        Ok(s)
    }

    /// Makes the file at `path` hold `bytes`, and nothing else: it's made
    /// if there's none, and what it held is gone if there is. If it fails,
    /// the file may hold part of `bytes`.
    export fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Error> {
        checked(path)?;
        crate::io::status(self.os_write(path, bytes, false))
    }

    /// Adds `bytes` to the end of the file at `path`, which is made if
    /// there's none. If it fails, part of `bytes` may have been added.
    export fn append(&self, path: &str, bytes: &[u8]) -> Result<(), Error> {
        checked(path)?;
        crate::io::status(self.os_write(path, bytes, true))
    }

    /// Removes the file at `path`: `NotFound` if there's none.
    export fn remove(&self, path: &str) -> Result<(), Error> {
        checked(path)?;
        crate::io::status(self.os_remove(path))
    }

    /// Opens the file at `path`, with no `\0` in it, to be read: it, or
    /// the operating system's failure, as `std::io::failure` reads it.
    #[intrinsic]
    fn os_open(&self, path: &str) -> Result<ReadFile, u64> with alloc;

    /// Writes all of `bytes` to the file at `path`, with no `\0` in it,
    /// after what it holds if `append`, or else instead of it: 0, or a
    /// failure, as `std::io::status` reads it.
    #[intrinsic]
    fn os_write(&self, path: &str, bytes: &[u8], append: bool) -> u64;

    /// Removes the file at `path`, with no `\0` in it: 0, or a failure.
    #[intrinsic]
    fn os_remove(&self, path: &str) -> u64;
}

/// `path` can be given to the operating system: it has no `\0`, which
/// would end it there, so that another file would be the one used.
fn checked(path: &str) -> Result<(), Error> {
    for b in path.as_bytes().iter() {
        if *b == 0u8 {
            return Err(Error::new(ErrorKind::InvalidInput));
        }
    }
    Ok(())
}

impl ReadFile {
    /// The next line of the file, without its line break, in `line`,
    /// instead of what it held: `alloc::io::read_line`. `Ok(false)` if
    /// there's no more.
    #[blame_caller]
    export fn read_line(&mut self, line: &mut String) -> Result<bool, Error> with alloc {
        alloc::io::read_line(self, line)
    }

    /// Reads all of the file that's left onto the end of `buf`, and gives
    /// how many bytes that was: `alloc::io::read_to_end`.
    #[blame_caller]
    export fn read_to_end(&mut self, buf: &mut List<u8>) -> Result<usize, Error> with alloc {
        alloc::io::read_to_end(self, buf)
    }

    /// Reads all of the file that's left onto the end of `s`, which must
    /// be UTF-8: `alloc::io::read_to_string`.
    #[blame_caller]
    export fn read_to_string(&mut self, s: &mut String) -> Result<usize, Error> with alloc {
        alloc::io::read_to_string(self, s)
    }

    /// Reads into `buf`, as `Read::read`: how many bytes, or a failure, as
    /// `std::io::failure` reads it.
    #[intrinsic]
    fn os_read(&mut self, buf: &mut [u8]) -> u64;

    /// Closes the file, and frees its buffer.
    #[intrinsic]
    fn os_close(self);
}

/// Dropping a file closes it. Closing a file that's only been read can't
/// lose anything, so whether it worked doesn't matter.
impl Discard for ReadFile {
    fn discard(self) { self.os_close() }
}

/// The file's bytes, from where reading is up to. A read gives what's in
/// the buffer, or else what one read of the operating system's gives.
impl Read for ReadFile {
    effect e = {};

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let r = self.os_read(buf);
        match crate::io::failure(r) {
            Some(e) => Err(e),
            None => Ok(core::num::wrap(r)),
        }
    }
}
