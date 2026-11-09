// The result writes only `T`s, but a bound on `T` can make a `&mut T` out
// of a place that isn't one: `pick` gives back the `&mut usize` it's
// given. So a method whose `T` has such a bound can't promise what's left
// once its borrow ends; with `T` a `usize`, `*c.slot() = 99` changes `n`.
trait Pick {
    fn pick(n: &mut usize) -> &mut Self;
}

impl Pick for usize {
    fn pick(n: &mut usize) -> &mut usize { n }
}

struct Counter<T> {
    n: usize,
    v: T,
}

impl<T: Pick> Counter<T> {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn slot(&mut self) -> &mut T ensures self.get() == old(self.get()) { T::pick(&mut self.n) }
}

fn main() {
    let mut c = Counter { n: 1, v: 0usize };
    if c.get() == 1 {
        *c.slot() = 99;
        let n: { n: usize | n == 1 } = c.get();
        let _ = n;
        let _ = c.v;
    }
}
// error: `ensures` can't name `self` at return
