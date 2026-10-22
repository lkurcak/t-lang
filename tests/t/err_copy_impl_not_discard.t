// The same for a written impl.
struct Point { x: i64, y: i64 }

impl Copy for Point {}

fn main() {}
// error: `Point` doesn't implement `Discard`, which `Copy` needs, as it's `trait Copy: Discard`
