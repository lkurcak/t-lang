// A call whose result can hold the `&mut` it's lent lends it for longer
// than the call: writes through the result change the place.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn field(p: &mut P) -> &mut i64 { &mut p.x }

fn f(mut p: P) -> i64 {
    if p.x > 0 {
        let r = field(&mut p);
        *r = 0;
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
