// Lints know associated types: their names are types' names, in
// `UpperCamelCase`, and a variable of a projection's type is a variable
// like any other.
trait Tr {
    type item: Discard;

    fn out(&self) -> Self::item;
}

#[derive(Discard)]
struct A { n: int }

impl Tr for A {
    type item = int;

    fn out(&self) -> int { self.n }
}

fn f<T: Tr>(t: &T) -> int {
    let x = t.out();
    1
}

fn main() { print(f(&A { n: 1 })); } // expect: 1
// warning: associated type `item` should have an upper camel case name
// warning: unused variable `x`
