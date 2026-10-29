// The second time round, the field has the value the first gave it.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P) -> i64 with {} {
    let mut n = 0;
    if p.x > 0 {
        while n < 2 {
            n = 10 / p.x;
            p.x = 0;
        }
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(P { x: 1 }))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
