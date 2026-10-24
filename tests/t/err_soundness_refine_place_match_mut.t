// A pattern binding by `&mut` through a `&mut` reference writes the
// field.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(r: &mut P) -> i64 {
    if r.x > 0 {
        match r {
            P { x } => *x = 0,
        }
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
