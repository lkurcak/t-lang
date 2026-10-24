// An `unsafe` block around nothing unsafe takes on no obligations, and
// only hides where the unsafe operations aren't.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    // SAFETY: nothing to keep.
    let n = unsafe { 1i64 };
    out.line(&n)?; // expect: 1
    Result::Ok(())
}
// warning: this `unsafe` block has nothing unsafe in it
