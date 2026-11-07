// A refinement of an enum's payload holds only of the variant that's
// there: `v < n` can't hold when `n` is 0, but `None` is what's given then,
// and the arm for it learns nothing it could prove anything with.
fn below(n: u8) -> Option<{ v: u8 | v < n }> {
    if n > 0 { Some(n - 1) } else { None }
}

fn f() -> u8 {
    let a: [u8; 1] = [7];
    match below(0) {
        Some(_) => 0,
        None => a[5],
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f())?;
    Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
