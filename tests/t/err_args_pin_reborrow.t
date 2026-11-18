// A `&mut` argument's reborrow, `&mut *g`, waits for the other arguments
// (docs/06, lemma A4 of docs/81). If one of them gives `g` another value,
// which place it borrows would depend on when it's found: so that's
// refused, and either way the program means the same (lemma A5 of
// docs/83).
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn f(a: &mut Game, b: &mut Game) {
    let mut g = a;
    put(g, { g = b; 1 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `g` a new value here: an earlier argument borrows `*g`
