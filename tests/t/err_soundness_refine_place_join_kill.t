// After an `if` whose branch assigns the whole struct, its fields are
// either value, though that branch never read them.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P, q: P, c: bool) -> i64 {
    if p.x > 0 {
        if c {
            p = q;
        }
        10 / p.x
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(P { x: 1 }, P { x: 0 }, true))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
