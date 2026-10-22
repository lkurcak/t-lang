// A list needs a pointer to the rest of the list; without one it would be
// infinitely big. Pointers come in step 5.
#[derive(Discard)]
enum List { Nil, Cons(i64, List) }
fn main() {}
// error: `List` contains itself, so it would have infinite size
