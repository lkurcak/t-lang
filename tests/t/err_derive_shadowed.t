// `#[derive(Eq)]` names a trait like any path does, and here `Eq` is this
// program's, which can't be derived.
trait Eq {}

#[derive(Eq)]
struct S {}

fn main() {}
// error: only `core`'s `Discard`, `Copy` and `Eq` can be derived, and `Eq` here is `crate::Eq`
