// Not even the parameters of the function it's in. A recursive one can't be
// a closure, since a closure's name isn't in scope in its own body, so
// there's no fix.
fn count_down(from: i64) -> i64 {
    fn go(n: i64) -> i64 {
        if n == from { 0 } else { go(n + 1) }
    }
    go(0)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&count_down(3))?;
    Result::Ok(())
}
// error: can't capture `from` in a nested `fn`; use a closure
