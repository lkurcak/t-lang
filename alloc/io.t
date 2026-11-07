//! Reading what a `core::io::Read` gives into lists and strings: a line
//! at a time, or all of it.

use core::io::{Error, ErrorKind, Read};
use core::num::wrapping_add;

use crate::list::List;
use crate::string::String;

/// Reads the next line from `r`, without its line break (`\n`, or
/// `\r\n`), into `line`, instead of what it held. `Ok(false)` if there's no
/// more input. The last line needn't end in a line break.
///
/// It reads a byte at a time, so it never reads past the line's end: what
/// `r` gives next is the next line, whatever `r` is. That's quick if `r`
/// keeps a buffer, as `std::io::Stdin` does.
///
/// A line that isn't UTF-8 is an error of kind `InvalidData`; it's read,
/// and `line` is left empty.
#[blame_caller]
export fn read_line<R: Read>(r: &mut R, line: &mut String) -> Result<bool, Error>
with {alloc, R::e}
{
    line.clear();
    let mut bytes = List::new();
    let mut any = false;
    let mut b = [0u8; 1];
    loop {
        if r.read(b.as_mut_slice())? == 0 {
            break;
        }
        any = true;
        if b[0] == 0x0A {
            break;
        }
        bytes.push(b[0]);
    }
    match bytes.pop() {
        Some(0x0D) => {},
        Some(b) => bytes.push(b),
        None => {},
    }
    match String::from_utf8(bytes) {
        Ok(s) => {
            line.push_str(s.as_str());
            Ok(any)
        },
        Err(_) => Err(Error::new(ErrorKind::InvalidData)),
    }
}

/// Reads all that's left of `r`, to the end of its input, onto the end of
/// `buf`, and gives how many bytes that was.
#[blame_caller]
export fn read_to_end<R: Read>(r: &mut R, buf: &mut List<u8>) -> Result<usize, Error>
with {alloc, R::e}
{
    let mut chunk = [0u8; 256];
    let mut total = 0usize;
    loop {
        let n = r.read(chunk.as_mut_slice())?;
        if n == 0 {
            return Ok(total);
        }
        let mut i = 0usize;
        while i < n && i < chunk.len() {
            buf.push(chunk[i]);
            i = i + 1;
        }
        total = wrapping_add(total, n);
    }
}

/// Reads all that's left of `r`, to the end of its input, onto the end of
/// `s`, and gives how many bytes that was. If it isn't UTF-8, that's an
/// error of kind `InvalidData`, and `s` is as it was.
#[blame_caller]
export fn read_to_string<R: Read>(r: &mut R, s: &mut String) -> Result<usize, Error>
with {alloc, R::e}
{
    let mut bytes = List::new();
    let n = read_to_end(r, &mut bytes)?;
    match String::from_utf8(bytes) {
        Ok(read) => {
            s.push_str(read.as_str());
            Ok(n)
        },
        Err(_) => Err(Error::new(ErrorKind::InvalidData)),
    }
}
