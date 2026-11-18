// Lending the reference `&mut` to a call that may change it.
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn redirect<'a>(g: &mut &'a mut Game, b: &'a mut Game) -> u32 {
    *g = b;
    1
}

fn f<'a>(a: &'a mut Game, b: &'a mut Game) {
    let mut g = a;
    put(g, redirect(&mut g, b));
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't lend `g` here: an earlier argument borrows `*g`
