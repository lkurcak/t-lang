// So does reborrowing what it points to.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(p: &mut P) { p.x = 0; }

fn f(r: &mut P) -> i64 {
    if r.x > 0 {
        zero(&mut *r);
        10 / r.x
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut p = P { x: 1 };
    out.line(&f(&mut p))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
