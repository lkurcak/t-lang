// The closure's result borrows `n`, but `get` says it's `'static`, and
// returns it out. A closure's type is invariant in its lifetimes, so the
// `'static` reaches what it borrows. (If its signature's lifetimes were
// free to vary apart from its fields', this would be a use after free.)
fn get<F: Fn() -> &'static i64 + Discard>(f: F) -> &'static i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s: &i64;
    {
        let n = 1;
        s = get(|| &n);
    }
    out.line(&(*s))?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
