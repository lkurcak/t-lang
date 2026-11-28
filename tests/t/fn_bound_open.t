// An `Fn` bound with no `with` leaves what its calls do open, `F::e`: a
// helper whose effects are inferred does what its closure does, with
// nothing written, and one that says its effects says `F::e`
// (docs/104-open-fn-bounds.md).
use alloc::boxed::Box;

fn twice<T: Discard, F: FnMut() -> T + Discard>(mut f: F) -> (T, T) { (f(), f()) }

// Its closure does nothing here, so nor does this call of `twice`.
fn quiet() -> u8 with {} {
    let (a, b) = twice(|| 2u8);
    a / b
}

export fn apply<F: Fn(i64) -> i64 + Discard>(f: F, x: i64) -> i64 with F::e { f(x) }

// Said where it's bounded by `with`: what the bound says.
fn bounded<F: Fn() -> u8 with panic + Discard>(f: F) -> u8 with F::e { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let (x, y) = twice(|| Box::new(20u8));
    out.line(&(*x + *y))?; // expect: 40
    out.line(&quiet())?; // expect: 1
    out.line(&apply(|x| x + 1, 2))?; // expect: 3
    // The open bound's effect parameter isn't written: `twice` takes two.
    out.line(&twice::<u8, _>(|| 5u8).1)?; // expect: 5
    out.line(&bounded(|| 6u8))?; // expect: 6
    Ok(())
}
