// A field of what a refined `&mut` points to can't be borrowed `&mut`: a
// write through the new reference changes the whole, whose refinement
// nothing would check, as a part of a refined variable can't be.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn f(o: &mut { s: S | s.a < s.b }) {
    let r = &mut (*o).a;
    *r = 100;
}

fn main() {
    let mut s: { s: S | s.a < s.b } = S { a: 1, b: 2 };
    f(&mut s);
}
// error: can't borrow `(*o).a` as `&mut`: it's in `(*o)`, whose type has a refinement, which a write through the reference could break
