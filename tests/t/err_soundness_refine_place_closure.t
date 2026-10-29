// A closure that captures the struct can change its fields when it's
// called.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P) -> i64 with {} {
    if p.x > 0 {
        let mut c = || p.x = 0;
        c();
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
