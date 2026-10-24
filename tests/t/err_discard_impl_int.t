// `Discard` is built into `i64`.
impl Discard for i64 {}

fn main() {}
// error: `Discard` can only be implemented for a struct or enum: it's built into `i64` already
