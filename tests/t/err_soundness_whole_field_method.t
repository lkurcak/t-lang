// A method taking `&mut self` of a field of what a refined `&mut` points
// to borrows the field `&mut`, which can't be.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

impl S {
    fn bump(&mut self) { self.a = 100; }
}

#[derive(Copy, Discard)]
struct W { x: S, y: u8 }

fn f(o: &mut { w: W | w.x.a < w.y }) {
    o.x.bump();
}

fn main() {
    let mut w: { w: W | w.x.a < w.y } = W { x: S { a: 1, b: 2 }, y: 3 };
    f(&mut w);
}
// error: can't borrow `o.x` as `&mut`: it's in `o`, whose type has a refinement, which a write through the reference could break
