// A field a refinement names, lent to a trait's method, is proved again
// when the call returns: the method may have changed it.
trait Shrink {
    #[measure]
    fn size(&self) -> usize;

    fn shrink(&mut self);
}

struct Kept<T: Shrink> { of: T, used: { u: usize | u <= of.size() } }

impl<T: Shrink> Kept<T> {
    fn less(&mut self) {
        self.of.shrink();
    }
}

fn main() {}
// error: can't prove `u <= of.size()` for field `used` of `Kept`, which names `of`
