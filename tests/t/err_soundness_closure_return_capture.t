// What a closure returns of what it captured lives as long as that does,
// not as long as its argument: `r` can't outlive `n`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = 1;
    let r;
    {
        let n = 2;
        let f = |_x: &i64| -> &i64 { &n };
        r = f(&a);
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
