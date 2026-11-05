// The format string borrows what its holes hold for as long as it's
// used, so a later hole can't change what an earlier one borrows.
fn bump(n: &mut i64) -> i64 {
    *n = core::num::wrapping_add(*n, 1);
    *n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n = 1i64;
    out.line(&f"{n} {bump(&mut n)}")?;
    Ok(())
}
// error: cannot borrow `n` as mutable while `n` is borrowed
