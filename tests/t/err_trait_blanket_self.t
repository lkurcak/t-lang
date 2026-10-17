trait A {}
impl<T: A> A for T {}
fn main() {}
// error: proving that a type implements `A` would go on forever: `impl<T: A> A for T` needs it to prove it
