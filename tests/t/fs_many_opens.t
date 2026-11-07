// Each file dropped is closed: opening one more times than a process may
// have files open at once works, as each is closed before the next.
use core::io::Read;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut out = os.stdout;
    let mut i = 0u32;
    let mut bytes = 0usize;
    while i < 3000 {
        let mut f = os.fs.open("one.txt")?;
        let mut b = [0u8; 8];
        bytes = core::num::wrapping_add(bytes, f.read(b.as_mut_slice())?);
        i = core::num::wrapping_add(i, 1);
    }
    out.line(&bytes) // expect: 12000
}
// file: one.txt=abc
