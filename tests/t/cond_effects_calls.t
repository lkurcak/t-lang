// A function may say when it has an effect: `with panic if P`. A call has
// it only where what's known there doesn't rule `P` out, as an operator
// panics only where it isn't proved not to (docs/90-conditional-effects.md).
// So `main` says no `panic`, though it unwraps.
use alloc::list::List;

fn sure(x: i64) -> Result<i64, !> { Ok(x) }

/// At most `limit`, or a panic if `x` is more.
fn capped(x: u8, limit: u8) -> u8 with panic if x > limit {
    if x > limit {
        panic("too big");
    }
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    // A `Result` whose error type is `!` is never an `Err`.
    out.line(&sure(1).unwrap())?;             // expect: 1
    out.line(&sure(2).expect("can't fail"))?; // expect: 2
    // Once it's known which it is.
    let o: Option<i64> = Some(3);
    if o.is_some() {
        out.line(&o.unwrap())?; // expect: 3
    }
    let r: Result<i64, bool> = Ok(4);
    if !r.is_err() {
        out.line(&r.expect("checked"))?; // expect: 4
    }
    // What a function ensures says it.
    let mut l = List::new();
    l.push(5i64);
    out.line(&l.pop().unwrap())?; // expect: 5
    // A function's own condition, ruled out.
    out.line(&capped(6, 10))?; // expect: 6
    let n = 7u8;
    out.line(&capped(n, n))?; // expect: 7
    l.discard();
    Ok(())
}
