// An array of references holds them: it can't outlive what they point to.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [&i64; 1];
    {
        let x: i64 = 1;
        a = [&x];
    }
    out.line(a[0])?;
    Result::Ok(())
}
// error: `x` doesn't live long enough: it goes out of scope here while still borrowed
