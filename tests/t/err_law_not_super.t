// A law is of a supertrait's method, or of one its `where` names
// (docs/88-iterator-laws.md).
trait A {
    fn f(&self) -> usize;
}

trait B {
    law fn f(&self) -> { v: usize | v < 3 };
}

fn main() {}
// error: no supertrait of `B` has a method `f`
