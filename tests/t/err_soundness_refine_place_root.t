// A reference that's assigned points somewhere else: what was known
// through it isn't any more.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(a: &P, b: &P) -> i64 {
    let mut r = a;
    if r.x > 0 {
        r = b;
        10 / r.x
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(&P { x: 1 }, &P { x: 0 }))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
