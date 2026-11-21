// A closure's effects are its body's sites', and a call whose condition is
// ruled out there has none: so this closure is `Fn() -> i64` with no
// effects, as `call` needs (docs/90-conditional-effects.md).
fn call<F: Fn() -> i64 + Discard>(f: F) -> i64 { f() }

fn sure(x: i64) -> Result<i64, !> { Ok(x) }

// A function that isn't exported and says no `with` has its body's
// effects, which are none here.
fn inferred(o: Option<i64>) -> i64 {
    match o.is_some() {
        true => o.unwrap(),
        false => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&call(|| sure(1).unwrap()))?; // expect: 1
    out.line(&inferred(Some(2)))?;         // expect: 2
    out.line(&inferred(None))?;            // expect: 0
    Ok(())
}
