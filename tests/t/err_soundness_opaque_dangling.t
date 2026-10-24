// What `reader` returns holds `x`, so it can't outlive `n`: its type
// borrows for `'a`, and the borrow checker sees that wherever it goes.
fn reader<'a>(x: &'a i64) -> impl Fn() -> i64 + Discard { move || *x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r = {
        let n = 3;
        reader(&n)
    };
    out.line(&r())?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
