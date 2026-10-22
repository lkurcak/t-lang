// Nor can a trait be implemented for a slice: its methods' `Self` would be
// a type parameter standing for one.
trait Size {
    fn size(&self) -> int;
}

impl Size for [int] {
    fn size(&self) -> int { self.len() }
}

fn main() {}
// error: a slice `[int]` can only be behind a reference, as in `&[int]`
