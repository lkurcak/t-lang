// Giving the whole the reference is in a new value changes it too.
#[derive(Discard)]
struct Game { x: u32 }

#[derive(Discard)]
struct Hold<'a> { g: &'a mut Game }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn f<'a>(a: &'a mut Game, b: &'a mut Game) {
    let mut h = Hold { g: a };
    put(h.g, { h = Hold { g: b }; 1 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `h` a new value here: an earlier argument borrows `*h.g`
