// An impl's consumer bound on its parameter, instantiated by a `&mut`:
// the impl's methods can write through it.
use alloc::list::List;

trait Go {
    fn go(self);
}

impl<'a> Go for &'a mut usize {
    fn go(self) {
        *self = 99;
    }
}

trait Run {
    fn run(self);
}

#[derive(Discard)]
struct Hold<G> {
    g: G,
}

impl<G: Go> Run for Hold<G> {
    fn run(self) {
        self.g.go();
    }
}

fn start<R: Run>(r: R) {
    r.run();
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        start(Hold { g: &mut l[0] });
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what `start` may write behind argument `r`
