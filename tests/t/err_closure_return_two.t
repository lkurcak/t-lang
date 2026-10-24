// A closure's result can borrow from one of its parameters, but not
// either: each call picks how long each lives, apart, so its type would
// have to say the result lives as long as both, which a type can't.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let max = |x: &i64, y: &i64| if *x > *y { x } else { y };
    let a = 1;
    out.line(&(*max(&a, &a)))?;
    Result::Ok(())
}
// error: this closure's result could borrow from `x` or from `y`
