// Only a struct's field's refinement can end `where`.
fn f<T: Discard>(x: { v: u8 | v > 0 where T: Copy }, t: T) -> u8 {
    t.discard();
    x
}

fn main() {}
// error: only a struct's field's own refinement can end `where`, not one of what's in it, nor any other
