// An `io::Error` is its kind, and the operating system's number for it, if
// it came from there, which its text shows. It's one type for every
// stream, `Copy`, and comparable.
use core::io::{Error, ErrorKind};

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let e = Error::os(ErrorKind::NotFound, 2);
    out.line(&e)?;                        // expect: not found (os error 2)
    out.line(&e.os_code().unwrap_or(0))?; // expect: 2
    let f = Error::new(ErrorKind::UnexpectedEnd);
    out.line(&f)?;                      // expect: unexpected end of input
    out.line(&f.os_code().is_none())?;  // expect: true
    out.line(&(e == e))?;               // expect: true
    out.line(&(e.kind() == f.kind()))?; // expect: false
    out.line(&f"{e:?}")?;               // expect: Error { kind: NotFound, code: 2 }
    Err(Error::os(ErrorKind::BrokenPipe, 32))
}
// stderr: error: broken pipe (os error 32)
