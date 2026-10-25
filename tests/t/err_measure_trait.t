// A trait's method can't be a measure: an impl's body would have to be
// the trait's.
trait Sized2 {
    #[measure]
    fn len(&self) -> usize;
}

fn main() {}
// error: a trait's method can't be a measure
