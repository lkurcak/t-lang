// The reborrow of a `&mut` argument waits for the others, but a borrow an
// earlier argument keeps is still live then: two `&mut`s to one game.
#[derive(Discard)]
struct Game { b: [u8; 4] }

fn cell(g: &mut Game) -> &mut u8 { &mut g.b[0] }

fn both(g: &mut Game, c: &mut u8) {
    g.b[0] = 1;
    *c = 2;
}

fn play(g: &mut Game) {
    both(g, cell(g));
}

fn main() {
    let mut g = Game { b: [1, 2, 3, 4] };
    play(&mut g);
}
// error: cannot borrow `*g` as mutable more than once at a time: `*g` is already mutably borrowed
