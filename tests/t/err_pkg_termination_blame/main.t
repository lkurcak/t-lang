// A package that checked stays checked: a way back that's new goes
// through one of the program's impls, which is what's at fault.
struct V<T> { t: T }
impl<T: base::A> base::B for V<T> {} // error: proving that a type implements `base::A` could go on forever: `impl base::A for W<I>` needs `I::Item: base::B`
fn main() {}
