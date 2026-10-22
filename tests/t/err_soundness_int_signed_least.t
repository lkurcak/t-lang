// Of a `T: Signed`, the least is -1 minus the greatest, so `-x` doesn't
// fit at the least.
fn abs<T: Signed>(x: T) -> T { if x < 0 { -x } else { x } }

fn main() {}
// error: `-` may panic, but `abs` doesn't say `with panic`
