// What a `&mut` says of the whole of what it points to holds after a write
// to one of its fields too: `a < b` of the struct with `a` written.
// (Before step 66 only what was declared of the field itself was checked,
// and this left `S { a: 100, b: 2 }` behind the caller's refined `s`.)
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn f(o: &mut { s: S | s.a < s.b }) {
    (*o).a = 100;
}

fn main() {
    let mut s: { s: S | s.a < s.b } = S { a: 1, b: 2 };
    f(&mut s);
}
// error: can't prove `s.a < s.b` for `(*o)`, once `(*o).a` is written
