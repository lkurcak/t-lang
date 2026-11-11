// Nor straight to a call: what the callee writes there changes the whole,
// and only the place lent is proved again after the call.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn set(x: &mut u8) { *x = 100; }

fn f(o: &mut { s: S | s.a < s.b }) {
    set(&mut (*o).a);
}

fn main() {
    let mut s: { s: S | s.a < s.b } = S { a: 1, b: 2 };
    f(&mut s);
}
// error: can't borrow `(*o).a` as `&mut`: it's in `(*o)`, whose type has a refinement, which a write through the reference could break
