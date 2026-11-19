// A law isn't a measure: it's of another trait's method.
trait A {
    fn f(&self) -> usize;
}

trait B: A {
    #[measure]
    law fn f(&self) -> usize;
}

fn main() {}
// error: a law can't be a measure
