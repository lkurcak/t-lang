// The `Fn` traits are built in, and only closures and functions implement
// them.
#[derive(Discard)]
struct S {}

impl Fn for S {}

fn main() {}
// error: `Fn` is built in
