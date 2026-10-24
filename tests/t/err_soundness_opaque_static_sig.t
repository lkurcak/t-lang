// `pick`'s closure returns a `&'a i64`, and `get` wants one that returns a
// `&'static i64`: that makes `'a` `'static`, and `n` doesn't live that long.
fn pick<'a>(x: &'a i64) -> impl Fn() -> &'a i64 + Discard { move || x }

fn get<F: Fn() -> &'static i64 + Discard>(f: F) -> &'static i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s: &i64;
    {
        let n = 1;
        s = get(pick(&n));
    }
    out.line(&(*s))?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
