// An opaque result of a program's own trait, whose associated type gives a
// `&mut` to a refined field.
trait Give {
    type Out;
    fn give(self) -> Self::Out;
}

#[derive(Discard)]
struct W<'a, T> { r: &'a mut T }

impl<'a, T> Give for W<'a, T> {
    type Out = &'a mut T;
    fn give(self) -> &'a mut T { self.r }
}

fn mk<'a>(x: &'a mut { v: u8 | v < 10 }) -> impl Give<Out = &'a mut u8> { W { r: x } }

#[derive(Copy, Discard)]
struct S { a: { v: u8 | v < 10 }, b: u8 }

fn main() {
    let mut s = S { a: 3, b: 0 };
    *mk(&mut s.a).give() = 200;
    let _y: { v: u8 | v < 10 } = s.a;
}
// error: can't prove `v < 10` of what may be written through the result of `mk`, `impl Give<Out = &'a mut u8>`: `Self::Out = &'a mut u8` can make or change a value of it
