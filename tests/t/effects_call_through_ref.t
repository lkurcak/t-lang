// A call through a reference to a function whose effects are a variable
// still to settle: how it gets at what it calls is known before they are,
// as effects don't change that (docs/64-phase-order.md).
fn coerce<effect e>(f: fn(i64) -> i64 with e) -> fn(i64) -> i64 with e { f }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let f = coerce(|x: i64| x / 2);
    let r = &f;
    out.line(&r(8))?; // expect: 4
    let g = coerce(|x: i64| x + 1);
    let s = &&g;
    out.line(&s(8))?; // expect: 9
    Ok(())
}
