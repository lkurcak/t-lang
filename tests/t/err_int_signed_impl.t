#[derive(Copy, Discard, Eq)]
struct S {}

impl Signed for S {}

fn main() {}
// error: `Signed` is built in: only the signed integer types implement it
