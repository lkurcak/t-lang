// A `ReadFile` is a `Read`: code generic over one reads a file, the input,
// or bytes in memory alike.
use core::io::{Bytes, Read};
use std::io::Error;

fn count<R: Read>(r: &mut R) -> Result<usize, Error> with R::e {
    let mut buf = [0u8; 3];
    let mut total = 0usize;
    loop {
        let n = r.read(buf.as_mut_slice())?;
        if n == 0 {
            return Ok(total);
        }
        total = core::num::wrapping_add(total, n);
    }
}

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut out = os.stdout;
    let mut input = os.stdin;
    let mut f = os.fs.open("data.bin")?;
    out.line(&count(&mut f)?)?;     // expect: 11
    out.line(&count(&mut f)?)?;     // expect: 0
    out.line(&count(&mut input)?)?; // expect: 4
    let mem = [1u8, 2, 3, 4, 5];
    out.line(&count(&mut Bytes::new(mem.as_slice()))?) // expect: 5
}
// file: data.bin=0123456789
// stdin: abc
