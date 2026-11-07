// A test's input and output aren't a terminal: raw mode and the size are
// `Unsupported`, and the error says the operating system's `ENOTTY`.
use std::io::{Error, ErrorKind};

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let mut input = os.stdin;
    match input.raw() {
        Ok(r) => {
            r.discard();
            out.line(&"raw")?
        },
        Err(e) => {
            out.line(&(e.kind() == ErrorKind::Unsupported))?; // expect: true
            out.line(&e)?                                     // expect: unsupported (os error 25)
        },
    }
    match out.size() {
        Ok(s) => out.line(&s.cols)?,
        Err(e) => out.line(&e)?, // expect: unsupported (os error 25)
    }
    // Input is still read as before.
    let mut line = alloc::string::String::new();
    let _ = input.read_line(&mut line)?;
    out.line(&line.as_str())?; // expect: hello
    line.discard();
    Ok(())
}
// stdin: hello
