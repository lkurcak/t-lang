// Entropy fills any slice, and two numbers it gives are (but for a
// chance of one in 2^64) different. `Entropy` is `Copy`, so threads each
// take one.
use std::entropy::Entropy;
use std::io::Error;

fn seed(e: Entropy) -> u64 {
    match e.u64() {
        Ok(n) => n,
        Err(_) => 0,
    }
}

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let e = os.entropy;
    let mut buf = [0u8; 600];
    e.fill(buf.as_mut_slice())?;
    let mut zeros = 0usize;
    for b in buf.as_slice().iter() {
        if *b == 0u8 {
            zeros = zeros + 1;
        }
    }
    out.line(&(zeros < 50))?; // expect: true
    let mut none = [0u8; 0];
    e.fill(none.as_mut_slice())?;
    let t = os.threads.spawn(|| seed(e));
    let here = seed(e);
    let there = t.join();
    out.line(&(here != there))?;      // expect: true
    out.line(&(e.u64()? != e.u64()?)) // expect: true
}
