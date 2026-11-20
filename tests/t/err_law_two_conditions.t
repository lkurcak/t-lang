// A law can be of one other trait's method, which its `where` names.
trait A {
    fn f(&self) -> usize;
}

trait C {}

trait B {
    law fn f(&self) -> { v: usize | v < 3 } where Self: A, Self: C;
}

fn main() {}
// error: a law can be of one other trait's method
