// The same for a written impl.
struct Point { x: int, y: int }

impl Copy for Point {}

fn main() {}
// error: `Point` doesn't implement `Discard`, which `Copy` needs, as it's `trait Copy: Discard`
