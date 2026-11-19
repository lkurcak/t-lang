// `Read::read` reads no more than there's room for, which generic code
// counts on (docs/86-trait-contracts.md): the last byte read is in `buf`.
use core::io::{Bytes, Read};

fn last<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<u8, std::io::Error> with R::e {
    let n = r.read(buf)?;
    if n == 0 {
        return Ok(0);
    }
    Ok(buf[n - 1])
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let data = [1u8, 2, 3, 4, 5];
    let mut b = Bytes::new(&data);
    let mut buf = [0u8; 3];
    out.line(&last(&mut b, &mut buf)?)?; // expect: 3
    out.line(&last(&mut &mut b, &mut buf)?)?; // expect: 5
    Ok(())
}
