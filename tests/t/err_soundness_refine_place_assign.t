// Assigning a field gives it a new value: what was known of the old one
// isn't of it.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P) -> i64 with {} {
    if p.x > 0 {
        p.x = 0;
        10 / p.x
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(P { x: 1 }))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
