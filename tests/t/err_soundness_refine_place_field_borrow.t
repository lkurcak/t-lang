// So can a `&mut` to the field.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn zero(r: &mut i64) { *r = 0; }

fn f(mut p: P) -> i64 with {} {
    if p.x > 0 {
        zero(&mut p.x);
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
