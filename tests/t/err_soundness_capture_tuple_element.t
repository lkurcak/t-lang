// A tuple's element may have a refinement relating the places in it,
// which a `let` without one declares from its value's type: `t.0.x` and
// `t.0.y` here. A closure changing `t.0.x` captures `t.0` whole, so what
// it writes is checked against `t.0` (K2a of
// docs/79-what-a-capture-splits.md). From 0.91.0 to 0.93.0 it captured
// `t.0.x` alone, and `need` below read a `t.0` that broke its refinement.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn make() -> ({ v: Q | v.x <= v.y }, u32) { (Q { x: 0, y: 10 }, 0) }

fn main() with {} {
    let mut t = make();
    let mut c = || t.0.x = 50; // error: can't prove `v.x <= v.y` for `t.0`, once `t.0.x` is written
    c();
    need(&t.0);
}
