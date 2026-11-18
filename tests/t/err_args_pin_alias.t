// Another `&mut` to the reference, used by a later argument, could change
// it: so it can't be in use where the argument is.
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn f<'a>(a: &'a mut Game, b: &'a mut Game) {
    let mut g = a;
    let gg = &mut g;
    put(g, { *gg = b; 1 });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: `g` is mutably borrowed
