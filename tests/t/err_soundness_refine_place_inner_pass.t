// A `&mut` that's a field, passed on, lets the callee write through it.
#[derive(Discard)]
struct W<'a> { r: &'a mut P }

#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(p: &mut P) { p.x = 0; }

fn f(w: W) -> i64 {
    if w.r.x > 0 {
        zero(w.r);
        10 / w.r.x
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut p = P { x: 1 };
    out.line(&f(W { r: &mut p }))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
