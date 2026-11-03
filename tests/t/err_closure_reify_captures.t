// Only a closure that captures nothing is a function: one that captures
// carries what it captures, which a function value has nowhere to keep.
// The same through a variable as for the closure itself.
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let k = 10;
    let f = |x: i64| x * k;
    out.line(&apply(f, 4))?;
    Result::Ok(())
}
// error: expected `fn(i64) -> i64`, found a closure that captures `k`
