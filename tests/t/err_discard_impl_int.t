// `Discard` is built into `int`.
impl Discard for int {}

fn main() {}
// error: `Discard` can only be implemented for a struct or enum
