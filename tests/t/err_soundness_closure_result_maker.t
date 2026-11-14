// A bound that makes values of `R` makes the closure's result one of many.
trait Mk {
    fn mk() -> Self;
}

impl Mk for u32 {
    fn mk() -> u32 { 0 }
}

fn call<R: Mk + Discard, F: FnOnce() -> R + Discard>(f: F) -> R {
    let _ = f();
    R::mk()
}

fn five() -> { v: u32 | v > 0 } { 5 }

fn main() {
    let c: { v: u32 | v > 0 } = call(|| five());
    let _ = c;
}
// error: can't prove `v > 0`
