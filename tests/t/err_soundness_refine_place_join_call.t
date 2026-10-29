// After an `if` whose branch lends the struct to a call, its field is
// whatever the call left, or what it was.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(p: &mut P) { p.x = 0; }

fn f(mut p: P, c: bool) -> i64 with {} {
    if p.x > 0 {
        if c {
            zero(&mut p);
        }
        10 / p.x
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(P { x: 1 }, true))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
