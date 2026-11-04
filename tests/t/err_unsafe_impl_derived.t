// A type is `Sync` by its fields or by its writer's promise, not both.
#[derive(Discard, Sync)]
struct P { x: u32 }

unsafe impl Sync for P {}
// error: `P` derives `Sync` already

fn main() {}
