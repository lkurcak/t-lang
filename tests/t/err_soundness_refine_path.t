// `T::m` without a call is a function value too.
#[derive(Discard)]
struct S {}

impl S {
    fn div(a: i64, d: { d: i64 | d > 0 }) -> i64 { a / d }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = S::div;
    out.line(&f(1, 0))?;
    Result::Ok(())
}
// error: `div` can't be used as a value: its parameter `d` has a refinement, which a call through a value wouldn't check
