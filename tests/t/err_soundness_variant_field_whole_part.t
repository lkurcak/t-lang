// Nor can a part of it be bound `&mut`: a write to the part is a write to
// the whole, which the reference to the part doesn't know of.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

#[derive(Copy, Discard)]
enum E { A({ s: S | s.a < s.b }) }

fn main() {
    let mut e = E::A(S { a: 1, b: 2 });
    match &mut e {
        E::A(S { a, .. }) => *a = 100,
    }
}
// error: can't bind `a` by `&mut`
