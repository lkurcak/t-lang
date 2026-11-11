// A field of a field: the write changes every value it's in, up to what
// the reference points to, and each must keep what's declared of it.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

#[derive(Copy, Discard)]
struct W { x: S, y: u8 }

fn f(o: &mut { w: W | w.x.a < w.y }) {
    (*o).x.a = 100;
}

fn main() {
    let mut w: { w: W | w.x.a < w.y } = W { x: S { a: 1, b: 2 }, y: 3 };
    f(&mut w);
}
// error: can't prove `w.x.a < w.y` for `(*o)`, once `(*o).x.a` is written
