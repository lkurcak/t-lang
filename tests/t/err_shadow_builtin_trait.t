// The built-in traits can't be shadowed: `==` and `#[derive]` mean them.
trait Discard {}

fn main() {}
// error: `Discard` is a built-in trait
