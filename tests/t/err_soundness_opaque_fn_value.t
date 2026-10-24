// Through a function value, what `reader` returns still borrows what its
// argument does: the function type is higher-ranked, and its result type
// is the opaque type at the lifetime of each call.
fn reader(x: &i64) -> impl Fn() -> i64 + Discard { move || *x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let g = reader;
    let f = {
        let n = 1;
        g(&n)
    };
    out.line(&f())?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
