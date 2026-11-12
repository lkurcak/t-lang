// A consumer's impl generic in an iterator of `&mut`s writes through what
// it gives.
use alloc::list::List;

trait Go {
    effect e;
    fn go(self) with Self::e;
}

#[derive(Discard)]
struct Wrap<I> { i: I }

impl<'a, I: Iterator<Item = &'a mut usize> + Discard> Go for Wrap<I> {
    effect e = I::e;
    fn go(self) with I::e {
        for r in self.i {
            *r = 99;
        }
    }
}

fn run<G: Go + Discard>(g: G) with G::e {
    g.go();
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    run(Wrap { i: l.iter_mut() });
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what `run` may write behind argument `g`: `G: Go` can make or change a value of it
