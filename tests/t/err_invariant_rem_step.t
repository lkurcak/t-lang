// `shift % 4 == 0` holds at the head only if each round keeps it: 3 down
// doesn't.
fn f() -> u32 {
    let mut shift: u32 = 64;
    while shift > 3 invariant shift % 4 == 0 {
        shift = shift - 3;
    }
    shift
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f())?;
    Ok(())
}
// error: can't prove `shift % 4 == 0` for the loop's invariant
