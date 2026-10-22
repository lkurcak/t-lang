// Two impls overlap if some type could be both of theirs: which one it
// used would be a choice.
trait Show { fn show(&self); }
#[derive(Discard)]
enum Option<T> { Some(T), None }
impl<T> Show for Option<T> { fn show(&self) {} }
impl Show for Option<i64> { fn show(&self) {} }
fn main() {}
// error: this impl of `Show` overlaps `impl Show for Option<T>`: a type can be both `Option<i64>` and `Option<T>`
