#[derive(Copy, Discard, Eq)]
struct S {}

impl Int for S {}

fn main() {}
// error: `Int` is built in: only the integer types implement it
