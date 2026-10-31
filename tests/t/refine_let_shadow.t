// A `let`'s refinement names what's in scope before its variable is: the
// `x` in `v > x` is the first `x`, which the interpreter checks the second
// against, not itself.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64 = 5;
    let x: { v: i64 | v > x } = 7;
    out.line(&x)?; // expect: 7
    Result::Ok(())
}
