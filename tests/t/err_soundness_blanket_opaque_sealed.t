// An `impl Tr` value's associated type is what its bounds say, never what
// a blanket impl says, unless that impl's bounds hold of it: here the
// hidden type, `Foo`, can't be `Small`, so its own impl is the one that
// runs, whose `get` gives a `bool` (hole O1, from 0.42.0 to 0.87.0).
sealed trait Small {}

impl Small for u8 {}

trait Tr {
    type A;

    fn get(&self) -> Self::A;
}

impl<T: Small> Tr for T {
    type A = i64;

    fn get(&self) -> i64 { 5 }
}

#[derive(Discard)]
struct Foo {}

impl Tr for Foo {
    type A = bool;

    fn get(&self) -> bool { true }
}

fn f() -> impl Tr + Discard { Foo {} }

fn main() {
    let x: i64 = f().get(); // error: expected `i64`, found `<impl Tr + Discard as Tr>::A`
    let _y = x + 1;
}
