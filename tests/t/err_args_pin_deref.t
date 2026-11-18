// Through a reference to the reference: `*rr` given a new value.
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn f(a: &mut Game, b: &mut Game) {
    let mut g = a;
    let rr = &mut g;
    put(*rr, { *rr = b; 2 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `*rr` a new value here: an earlier argument borrows `**rr`
