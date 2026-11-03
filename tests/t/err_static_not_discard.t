// A static is never used up, so its value must be one that may be thrown
// away.
use core::sync::Atomic;

#[derive(Sync)]
struct Token { n: Atomic<u32> }

static T: Token = Token { n: Atomic::new(0) };
// error: a static must be `Discard`, as it's never used up, and `Token` isn't

fn main() {}
