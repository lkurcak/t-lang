// A type parameter's associated type is a blanket impl's only if that
// impl's bounds hold of it: `T: Tr` doesn't say `T: Small`, and `get::<Foo>`
// runs `Foo`'s impl, whose `A` is `bool`.
trait Small {}

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

fn get<T: Tr>(x: &T) -> i64 {
    x.get() // error: expected `i64`, found `T::A`
}

fn main() {
    let _n = get(&Foo {});
}
