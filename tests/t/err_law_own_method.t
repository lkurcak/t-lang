// What a trait's own method promises is said where it's declared; a law is
// of another trait's (docs/88-iterator-laws.md).
trait A {
    fn f(&self) -> usize;

    law fn f(&self) -> { v: usize | v < 3 };
}

fn main() {}
// error: `f` is `A`'s own method
