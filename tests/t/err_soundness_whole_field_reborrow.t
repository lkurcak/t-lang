// The same through a reborrow of the whole: `r` says what `o` says.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn f(o: &mut { s: S | s.a < s.b }) {
    let r = &mut *o;
    r.a = 100;
}

fn main() {
    let mut s: { s: S | s.a < s.b } = S { a: 1, b: 2 };
    f(&mut s);
}
// error: can't prove `s.a < s.b` for `r`, once `r.a` is written
