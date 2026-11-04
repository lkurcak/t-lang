// An inherent impl promises nothing, so it can't be unsafe.
struct P { x: u32 }

unsafe impl P {}
// error: only `Send` and `Sync` can be implemented with `unsafe impl`

fn main() {}
