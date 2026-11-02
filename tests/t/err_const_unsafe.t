// Nor can it use `unsafe`: what it promises isn't checked while compiling.
unsafe fn trust() -> u8 { 1 }

const N: u8 = unsafe { trust() };
// error: a const's initializer can't have the effect `unsafe`

fn main() {}
