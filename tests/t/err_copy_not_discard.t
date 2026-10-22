// `trait Copy: Discard`, and `Discard` is never assumed.
#[derive(Copy)]
struct Point { x: i64, y: i64 }

fn main() {}
// error: `Point` can't be `Copy` unless it's `Discard`
