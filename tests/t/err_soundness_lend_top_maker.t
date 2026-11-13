// A trait whose method gives back a `Self` makes values: not a consumer.
trait Mk {
    fn mk() -> Self;
}

impl Mk for u32 {
    fn mk() -> u32 { 0 }
}

fn reset<T: Mk + Discard>(a: &mut T) { *a = T::mk(); }

fn main() {
    let mut a: { v: u32 | v > 0 } = 1;
    reset(&mut a);
    let _ = a;
}
// error: can't prove `v > 0` of what `reset` may write behind argument `a`: `T: Mk` can make or change a value of it
// error: can't prove `v > 0` for `a` after the call to `reset`
