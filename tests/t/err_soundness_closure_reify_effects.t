// A closure made a function keeps its effects: one that may panic isn't a
// function that can't.
fn may(x: i64) -> i64 with panic { x + 1 }
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let f = |x: i64| may(x);
    out.line(&apply(f, 1))?;
    Result::Ok(())
}
// error: expected `fn(i64) -> i64`, found `fn(i64) -> i64 with panic`
