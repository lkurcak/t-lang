// `G: Discard` lets generic code drop a `G`, which runs its `discard`.
use alloc::list::List;

struct W<'a, T> {
    r: &'a mut T,
}

impl<'a> Discard for W<'a, usize> {
    fn discard(self) {
        *self.r = 99;
    }
}

fn run<G: Discard>(_g: G) {}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        run(W { r: &mut l[0] });
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what `run` may write behind argument `_g`: `W<usize>`'s `discard` can make or change a value of it
