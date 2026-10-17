// Proving `T: A` needs `T: B`, which needs `T: A`: it would never end.
trait A {}
trait B {}
impl<T: A> B for T {}
impl<T: B> A for T {}
fn main() {}
// error: proving that a type implements `B` could go on forever: `impl<T: A> B for T` and `impl<T: B> A for T` need each other
