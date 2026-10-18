// `trait Copy: Discard`, and `Discard` is never assumed.
#[derive(Copy)]
struct Point { x: int, y: int }

fn main() {}
// error: `Point` can't be `Copy` unless it's `Discard`
