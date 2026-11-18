// A box on the way to the place is the same: given a new box, the place is
// another.
use alloc::boxed::Box;

#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = v; }

fn main() with alloc {
    let mut bx = Box::new(Game { x: 0 });
    put(&mut *bx, { bx = Box::new(Game { x: 1 }); 2 });
}
// error: can't give `bx` a new value here: an earlier argument borrows `*bx`
