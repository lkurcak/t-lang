// A method's receiver waits too: `g.set(..)` borrows `*g` after its
// arguments.
#[derive(Discard)]
struct Game { x: u32 }

impl Game {
    fn set(&mut self, v: u32) { self.x = v; }
}

fn f(a: &mut Game, b: &mut Game) {
    let mut g = a;
    g.set({ g = b; 1 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `g` a new value here: an earlier argument borrows `*g`
