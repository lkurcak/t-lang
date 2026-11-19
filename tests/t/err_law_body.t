// A law has no body: it says what each impl's method promises.
trait A {
    fn f(&self) -> usize;
}

trait B: A {
    law fn f(&self) -> { v: usize | v < 3 } { 2 }
}

fn main() {}
// error: a law has no body
