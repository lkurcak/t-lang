fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut r = &0i64;
    {
        let x = 1;
        r = &x;
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `x` doesn't live long enough: it goes out of scope here while still borrowed
