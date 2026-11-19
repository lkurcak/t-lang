// A law is written as the method's declared: its types, effects and bounds
// (docs/88-iterator-laws.md).
trait A {
    fn f(&self, k: usize) -> usize;
}

trait B: A {
    law fn f(&self, k: u32) -> { v: usize | v < 3 };
}

fn main() {}
// error: law `f` isn't written as `A` declares `f`: its type should be
