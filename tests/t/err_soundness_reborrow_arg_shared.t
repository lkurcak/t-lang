// The reborrow of a `&mut` argument waits for the others, but a shared
// borrow one of them is, which the call is given, is live at the call:
// the game would be both lent `&mut` and read through `c`.
#[derive(Discard)]
struct Game { b: [u8; 4] }

fn both(g: &mut Game, c: &u8) {
    g.b[0] = *c;
}

fn play(g: &mut Game) {
    both(g, &g.b[1]);
}

fn main() {
    let mut g = Game { b: [1, 2, 3, 4] };
    play(&mut g);
}
// error: cannot borrow `*g` as mutable while `g.b` is borrowed
