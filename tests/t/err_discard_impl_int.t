// `Discard` is built into `i64`, and can only be derived anyway.
impl Discard for i64 {}

fn main() {}
// error: `Discard` can only be derived
