// What can go wrong: no such file, a path with a `\0` in it, which never
// reaches the operating system, and text that isn't UTF-8.
use std::io::{Error, ErrorKind};

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let fs = os.fs;
    let mut out = os.stdout;
    match fs.open("missing.txt") {
        Ok(_) => out.line("opened")?,
        Err(e) => out.line(&e)?, // expect: not found (os error 2)
    }
    match fs.remove("missing.txt") {
        Ok(()) => out.line("removed")?,
        Err(e) => out.line(&e)?, // expect: not found (os error 2)
    }
    match fs.write("no/such/dir.txt", "x".as_bytes()) {
        Ok(()) => out.line("written")?,
        Err(e) => out.line(&(e.kind() == ErrorKind::NotFound))?, // expect: true
    }
    match fs.write("a\0b", "x".as_bytes()) {
        Ok(()) => out.line("written")?,
        Err(e) => out.line(&e)?, // expect: invalid input
    }
    let bad = [0x61u8, 0xFF];
    fs.write("bad.txt", bad.as_slice())?;
    match fs.read_to_string("bad.txt") {
        Ok(s) => s.discard(),
        Err(e) => out.line(&(e.kind() == ErrorKind::InvalidData))?, // expect: true
    }
    fs.remove("missing.txt")
}
// stderr: error: not found (os error 2)
