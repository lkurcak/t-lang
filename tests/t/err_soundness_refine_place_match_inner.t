// A pattern can look through a `&mut` inside the struct it matches, and
// bind what's behind it by `&mut`.
#[derive(Discard)]
struct W<'a> { r: &'a mut P }

#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(w: W) -> i64 with {} {
    if w.r.x > 0 {
        let W { r: P { x } } = w;
        *x = 0;
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
