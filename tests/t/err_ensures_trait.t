// A trait's methods can't have refinements yet, nor `ensures`.
trait Reset {
    fn reset(&mut self) ensures true;
}

fn main() {}
// error: `ensures` can't be on a trait's methods yet: an impl's would have to promise no less
