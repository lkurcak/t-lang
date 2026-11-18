// A shared borrow of a place waits too, and is found through `r` as well.
#[derive(Discard)]
struct Game { x: u32 }

fn show(g: &Game, v: u32) -> u32 { g.x + v }

fn f(a: &Game, b: &Game) -> u32 {
    let mut r = a;
    show(&*r, { r = b; 1 })
}

fn main() {
    let a = Game { x: 0 };
    let b = Game { x: 5 };
    let _ = f(&a, &b);
}
// error: can't give `r` a new value here: an earlier argument borrows `*r`
