// An opaque result that gives a `&Cell` to a refined `Cell`.
use core::cell::Cell;

trait Get {
    type Out;
    fn get(self) -> Self::Out;
}

#[derive(Copy, Discard)]
struct W<'a, T> {
    r: &'a T,
}

impl<'a, T> Get for W<'a, T> {
    type Out = &'a T;
    fn get(self) -> &'a T {
        self.r
    }
}

fn mk<'a>(c: &'a Cell<{ v: u8 | v < 10 }>) -> impl Get<Out = &'a Cell<u8>> {
    W { r: c }
}

fn main() {
    let c: Cell<{ v: u8 | v < 10 }> = Cell::new(3);
    mk(&c).get().set(200);
    let _y: { v: u8 | v < 10 } = c.get();
}
// error: can't prove `v < 10` of what may be written through the result of `mk`, `impl Get<Out = &'a Cell<u8>>`: `Self::Out = &'a Cell<u8>` can make or change a value of it
