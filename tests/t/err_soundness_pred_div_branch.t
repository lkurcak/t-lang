// What's known of a quotient on one path isn't on another: only what
// defines it holds everywhere. `a / 2 == 0` is known where it was tested.
fn f(a: u64, c: bool) -> { r: u64 | r == 0 } {
    if c {
        if a / 2 == 0 {
            return 0;
        }
    }
    a / 2
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(8, false))?;
    Ok(())
}
// error: can't prove `r == 0` for the result of `f`
