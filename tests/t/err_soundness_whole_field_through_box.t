// What a box holds is part of the value it's in: a write to a field of it
// changes what's declared of the whole outside the box too.
use alloc::boxed::Box;

#[derive(Discard)]
struct S { a: u8, b: u8 }

#[derive(Discard)]
struct W { bx: Box<S>, y: u8 }

fn f(o: &mut { w: W | w.bx.a < w.y }) {
    o.bx.a = 100;
}

fn main() with alloc {
    let mut w: { w: W | w.bx.a < w.y } = W { bx: Box::new(S { a: 1, b: 2 }), y: 3 };
    f(&mut w);
}
// error: can't prove `w.bx.a < w.y` for `o`, once `o.bx.a` is written
