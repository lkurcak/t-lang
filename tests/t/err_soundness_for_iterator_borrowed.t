// `for x in &mut it` borrows `it` for the whole loop: it can't be looked at
// in the body.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut it = 0..3i64;
    for i in &mut it {
        out.line(&it.start())?;
        out.line(&i)?;
    }
    Result::Ok(())
}
// error: cannot borrow `it` while `it` is mutably borrowed
