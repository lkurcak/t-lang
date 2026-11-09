// Bytes in memory are a `Read`: each read takes what fits from the front,
// and `read_exact` fails with `UnexpectedEnd` when they run out first.
use core::io::{Bytes, ErrorKind, Read};

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut r = Bytes::new("hello, world".as_bytes());
    let mut buf = [0u8; 5];
    let n = r.read(buf.as_mut_slice())?;
    out.line(&n)?;              // expect: 5
    out.line(&buf[0])?;         // expect: 104
    out.line(&r.rest().len())?; // expect: 7
    let mut seven = [0u8; 7];
    r.read_exact(seven.as_mut_slice())?;
    out.line(&seven[6])?;                    // expect: 100
    out.line(&r.read(buf.as_mut_slice())?)?; // expect: 0
    let mut more = Bytes::new("ab".as_bytes());
    match more.read_exact(buf.as_mut_slice()) {
        Ok(()) => out.line("read"),
        Err(e) => out.line(&(e.kind() == ErrorKind::UnexpectedEnd)), // expect: true
    }
}
