// `from_fn`'s closure is `FnMut`: it's called once for each value, and
// what it changes stays changed.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut n: u64 = 0;
    let a: [u64; 3] = core::array::from_fn(|_| {
        n += 10;
        n
    });
    out.line(&a[2])?; // expect: 30
    out.line(&n)?;    // expect: 30
    Result::Ok(())
}
