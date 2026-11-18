// Through a field holding the reference: `h.g` given a new value.
#[derive(Discard)]
struct Game { x: u32 }

#[derive(Discard)]
struct Hold<'a> { g: &'a mut Game }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn f(a: &mut Game, b: &mut Game) {
    let mut h = Hold { g: a };
    put(h.g, { h.g = b; 1 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `h.g` a new value here: an earlier argument borrows `*h.g`
