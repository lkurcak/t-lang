// Nor can a trait be implemented for `str`: its methods' `Self` would be a
// type parameter standing for one.
trait Size {
    fn size(&self) -> usize;
}

impl Size for str {
    fn size(&self) -> usize { self.len() }
}

fn main() {}
// error: `str` can only be behind a shared reference, as in `&str`
