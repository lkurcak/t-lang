// Nor a variant's field's.
enum E<T> { A(T, { v: u8 | v > 0 where T: Copy }) }

fn main() {}
// error: only a struct's field's own refinement can end `where`, not one of what's in it, nor any other
