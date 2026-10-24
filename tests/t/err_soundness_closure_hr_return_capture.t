// A closure that returns either its argument or what it captured has a
// result as long-lived as its argument, as the body says it borrows from
// it, and `&n` lasts only as long as `n`, so it can't promise that: `r`
// would outlive `n`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = 1;
    let r;
    {
        let n = 2;
        let f = |x: &i64| -> &i64 { if *x > 0 { x } else { &n } };
        r = f(&a);
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: this closure's result has the lifetime of `x`, which each call picks, so it could be any, even `'static`, but this only lives as long as the captured `n`
