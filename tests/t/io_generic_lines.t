// A program generic over where its input comes from and where its output
// goes: `main` hands it the streams, and its test bytes in memory, so the
// test sees exactly what a person at a terminal would (docs/47-io.md).
use alloc::io::read_line;
use alloc::list::List;
use alloc::string::String;
use core::io::{Bytes, Read, Write};
use std::io::Error;

fn shout<R: Read, W: Write>(input: &mut R, out: &mut W) -> Result<u32, Error>
with {alloc, R::e, W::e}
{
    let mut line = String::new();
    let mut n = 0u32;
    while read_line(input, &mut line)? {
        n = checked_add(n, 1).unwrap_or(n);
        out.put(&"> ")?;
        out.put(&line.as_str())?;
        out.line(&'!')?;
    }
    line.discard();
    Ok(n)
}

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut input = os.stdin;
    let mut out = os.stdout;
    let n = shout(&mut input, &mut out)?;
    out.line(&n)
}
// stdin: hi
// stdin: thére
// expect: > hi!
// expect: > thére!
// expect: 2

#[test]
fn shouts_each_line() -> Result<(), Error> with {alloc, panic} {
    let mut input = Bytes::new("a\r\nb c\nd".as_bytes());
    let mut out = List::new();
    assert_eq(&shout(&mut input, &mut out)?, &3);
    let text = String::from_utf8(out).ok().unwrap();
    assert(text == String::from("> a!\n> b c!\n> d!\n"));
    text.discard();
    Ok(())
}
// test: shouts_each_line ... ok
