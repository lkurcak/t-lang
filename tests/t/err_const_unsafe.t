// Nor can it use `unsafe`: what an `unsafe` block promises is promised to
// no one while compiling.
#[allow(unused_effects)]
fn trust() -> u8 with unsafe { 1 }

// SAFETY: nothing to keep.
const N: u8 = unsafe { trust() };
// error: a const's initializer can't use `unsafe`

fn main() {}
