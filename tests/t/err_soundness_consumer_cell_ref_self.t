// `&self` of a `Cell` is enough to set it.
use core::cell::Cell;

trait Look {
    fn look(&self);
}

impl Look for Cell<usize> {
    fn look(&self) {
        self.set(99);
    }
}

fn run<G: Look>(g: &G) {
    g.look();
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
// error: can't prove `d < 10` of what `run` may write behind argument `g`: `G: Look` can make or change a value of it
