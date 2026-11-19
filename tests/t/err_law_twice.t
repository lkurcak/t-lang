// One law of a method, saying all of it (docs/88-iterator-laws.md).
trait A {
    fn f(&self) -> usize;
}

trait B: A {
    law fn f(&self) -> { v: usize | v < 3 };

    law fn f(&self) -> { v: usize | v > 0 };
}

fn main() {}
// error: trait `B` states a law of `f` twice
