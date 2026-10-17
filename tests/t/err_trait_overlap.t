// Two impls overlap if some type could be both of theirs: which one it
// used would be a choice.
trait Show { fn show(&self); }
enum Option<T> { Some(T), None }
impl<T> Show for Option<T> { fn show(&self) {} }
impl Show for Option<int> { fn show(&self) {} }
fn main() {}
// error: this impl of `Show` overlaps `impl Show for Option<T>`: a type can be both `Option<int>` and `Option<T>`
