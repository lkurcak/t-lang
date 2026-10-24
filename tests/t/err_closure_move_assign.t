// A `move` closure has its own copy of `n`, but changing it still needs
// `n` to be `mut`, as it would anywhere.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 0;
    let mut c = move || {
        n = n + 1;
        n
    };
    out.line(&c())?;
    Result::Ok(())
}
// error: cannot assign to `n`, as `n` isn't declared `mut`
