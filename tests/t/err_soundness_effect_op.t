// `+` panics when it overflows, so a function that adds must say it may
// panic: `fn main()` with no `with` promises the program never does.
fn next(n: i64) -> i64 with {} { n + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&next(9223372036854775807))?;
    Result::Ok(())
}
// error: `+` may panic, but `next` doesn't say `with panic`
