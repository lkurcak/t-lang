// A consumer trait's impl for a `&Cell` can set it.
use core::cell::Cell;

trait Go {
    fn go(self);
}

impl<'a> Go for &'a Cell<usize> {
    fn go(self) {
        self.set(99);
    }
}

fn run<G: Go>(g: G) {
    g.go();
}

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

fn main() {
    let c: Cell<{ d: usize | d < 10 }> = Cell::new(1);
    run(&c);
    let _x = read(&c);
}
// error: can't prove `d < 10` of what `run` may write behind argument `g`: `G: Go` can make or change a value of it
