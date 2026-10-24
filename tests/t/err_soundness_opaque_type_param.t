// The type `hold` returns may hold its `T`: here a `&i64`, so it can't
// outlive `n`.
fn hold<T: Copy>(x: T) -> impl Fn() -> T + Discard { move || x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = {
        let n: i64 = 1;
        hold(&n)
    };
    out.line(&(*f()))?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
