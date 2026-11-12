// A consumer trait's impl for a `&mut` can write through it: the type
// parameter's instantiation, behind its `&mut`, is made by `G: Go`.
use alloc::list::List;

trait Go {
    fn go(self);
}

impl<'a> Go for &'a mut usize {
    fn go(self) {
        *self = 99;
    }
}

fn run<G: Go>(g: G) {
    g.go();
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        run(&mut l[0]);
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what `run` may write behind argument `g`: `G: Go` can make or change a value of it
