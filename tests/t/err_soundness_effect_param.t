// An effect parameter stands for whatever the caller's closure does, so
// calling it has that effect, which the function must say.
fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F) -> i64 { f(1) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&apply(|x| x / 0))?;
    Result::Ok(())
}
// error: `f` may have the effects `e`, but `apply` doesn't say `with e`
