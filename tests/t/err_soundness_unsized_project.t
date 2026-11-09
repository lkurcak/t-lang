// Nor through an associated type: `<&D as Tr>::A` isn't `Option<D>`,
// as no impl says what it is.
use core::fmt::Display;

trait Tr { type A; fn make(&self) -> Self::A; }

impl<'a, T: Display> Tr for &'a T {
    type A = Option<T>;
    fn make(&self) -> Option<T> { None }
}

fn f<'a, unsized D: Display>(x: &'a D) -> u8 {
    let _a = Tr::make(&x);
    0
}

fn main() {}
// error: doesn't implement `Tr`
