// A sealed trait may have a blanket impl of its own, on an open trait:
// then any type might implement it, and nothing is ruled out. Only what
// no impl's type can be is (see docs/29-closed-and-sealed-traits.md).
trait Open {}
sealed trait Small {}

impl<T: Open> Small for T {}

trait Describe {
    fn describe(&self) -> i64;
}

impl<T: Small> Describe for T {
    fn describe(&self) -> i64 { 1 }
}

impl Describe for char { // error: overlap
    fn describe(&self) -> i64 { 2 }
}

fn main() {}
