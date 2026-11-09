// As err_soundness_frame_bound_alias.t, with the measure in an impl of
// its own, which bounds `T` by nothing: the measure is free in `T`, but
// `slot`'s `T` isn't, as its `Pick` can make a `&mut T` of `n`.
trait Pick {
    fn pick(n: &mut usize) -> &mut Self;
}

impl Pick for usize {
    fn pick(n: &mut usize) -> &mut usize { n }
}

#[derive(Discard where T: Discard)]
struct Counter<T> {
    n: usize,
    v: T,
}

impl<T> Counter<T> {
    #[measure]
    fn get(&self) -> usize { self.n }
}

impl<T: Pick> Counter<T> {
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
