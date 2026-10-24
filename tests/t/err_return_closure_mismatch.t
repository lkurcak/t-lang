// The closure's result type is worked out from its `return` and its body,
// which must agree.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |x: i64| {
        if x > 0 { return true; }
        x
    };
    out.line(&f(1))?;
    Result::Ok(())
}
// error: this closure returns `bool`, but its body is `i64`
