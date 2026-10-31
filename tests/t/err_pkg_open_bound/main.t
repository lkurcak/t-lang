// A bound on what isn't a part of an impl's type is checked in the
// package that writes it, alone: it may only be of a closed trait. Before
// step 29 the library checked, and the program's impl below was blamed for
// the loop the two made together; now the library's impl is the error,
// whatever program it's in.
struct V<T> { t: T }
impl<T: base::A> base::B for V<T> {}
fn main() {}
