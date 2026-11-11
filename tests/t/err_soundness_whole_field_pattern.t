// Nor by a pattern, through match ergonomics.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn f(o: &mut { s: S | s.a < s.b }) {
    match o {
        S { a, .. } => *a = 100,
    }
}

fn main() {
    let mut s: { s: S | s.a < s.b } = S { a: 1, b: 2 };
    f(&mut s);
}
// error: can't bind `a` by `&mut`: it's in `(*o)`, whose type has a refinement, which a write through the reference could break
