// What an opaque type stands for can have an effect variable in it while
// the borrow checker looks, `fn(i64) -> i64 with e` for an `e` that's
// settled only after: what's kept for running has the set it settles on
// (docs/64-phase-order.md).
fn coerce<effect e>(f: fn(i64) -> i64 with e) -> fn(i64) -> i64 with e { f }

fn halver() -> impl Fn(i64) -> i64 + Discard { coerce(|x: i64| x / 2) }

fn apply<F: Fn(i64) -> i64 + Discard>(f: F, x: i64) -> i64 { f(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let h = halver();
    out.line(&h(8))?;               // expect: 4
    out.line(&apply(halver(), 6))?; // expect: 3
    Ok(())
}
