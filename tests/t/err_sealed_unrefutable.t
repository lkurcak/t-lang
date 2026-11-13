// A sealed trait may have a blanket impl of its own, on an open trait:
// then any type might implement it that another package could make
// `Iterator`, and that's not ruled out. Only what no impl's type can be,
// for good, is (see docs/29-closed-and-sealed-traits.md and
// docs/73-for-over-collections.md).
sealed trait Small {}

impl<T: Iterator> Small for T {}

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
