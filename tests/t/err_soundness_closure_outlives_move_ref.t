// `move` copies `r` into the closure, but `r` is a reference: the closure
// still can't outlive what it points to.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let c = {
        let n: i64 = 1;
        let r = &n;
        move || *r
    };
    out.line(&c())?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
