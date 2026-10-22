// Nor can a trait be implemented for a slice: its methods' `Self` would be
// a type parameter standing for one.
trait Size {
    fn size(&self) -> i64;
}

impl Size for [i64] {
    fn size(&self) -> i64 { self.len() }
}

fn main() {}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
