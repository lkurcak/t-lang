// A call lent a `&mut` in a loop changes the place for the next time
// round too.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(p: &mut P) { p.x = 0; }

fn f(mut p: P) -> i64 {
    let mut n = 0;
    if p.x > 0 {
        while n < 2 {
            n = 10 / p.x;
            zero(&mut p);
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
