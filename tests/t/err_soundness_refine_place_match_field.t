// A `match` on a struct holding a `&mut` can bind what's behind it by
// `&mut`, and write through that.
#[derive(Discard)]
struct W<'a> { r: &'a mut P }

#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(w: W) -> i64 with {} {
    if w.r.x > 0 {
        match w {
            W { r: P { x } } => *x = 0,
        }
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
