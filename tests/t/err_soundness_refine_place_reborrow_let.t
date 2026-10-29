// `let r2: &mut P = r;` reborrows `r`: writes through `r2` change `*r`,
// which is read again once `r2` is dead.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(r: &mut P) -> i64 with {} {
    if r.x > 0 {
        let r2: &mut P = r;
        r2.x = 0;
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
