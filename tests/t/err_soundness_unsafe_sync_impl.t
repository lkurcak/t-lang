// Nobody but core can say `Unsafe` is `Sync`: an `unsafe impl` is the
// promise of the package that declares the type.
use core::cell::Unsafe;
use core::sync::Sync;

unsafe impl Sync for Unsafe<u32> {}
// error: must be in the package that declares `Unsafe`

fn main() {}
