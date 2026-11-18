// The same, written: `&mut *g` is found when the borrow is taken, after the
// other arguments, so none may give `g` another value.
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn f(a: &mut Game, b: &mut Game) {
    let mut g = a;
    put(&mut *g, { g = b; 2 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `g` a new value here: an earlier argument borrows `*g`
