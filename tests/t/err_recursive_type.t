// A list needs a pointer to the rest of the list; without one it would be
// infinitely big. Pointers come in step 5.
enum List { Nil, Cons(int, List) }
fn main() {}
// error: `List` contains itself, so it would have infinite size
