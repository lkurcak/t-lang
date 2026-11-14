// `T: Int` makes values of `T` (its least, its greatest): so a call can't
// count on writing back only values it was given.
use core::num::{Int, least};

fn zero<T: Int>(a: &mut T) {
    *a = least::<T>();
}

fn main() {
    let mut a: { v: u32 | v > 0 } = 1;
    zero(&mut a);
    let _ = a;
}
// error: can't prove `v > 0` of what `zero` may write behind argument `a`: `T: Int` can make or change a value of it
// error: can't prove `v > 0` for `a` after the call to `zero`
