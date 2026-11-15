// A method taking `&mut self`, called in a closure on a place in a type
// argument's place: the closure captures `t.0` (K2a of
// docs/79-what-a-capture-splits.md), and lending `t.0.x` from it is
// refused, as it is outside a closure.
#[derive(Discard)]
struct C { n: u32 }

impl C {
    fn set(&mut self, v: u32) { self.n = v; }
}

#[derive(Discard)]
struct Q { x: C, y: u32 }

fn need(q: &{ v: Q | v.x.n <= v.y }) with {} {
    let d: u32 = q.y - q.x.n;
    let _ = d;
}

fn make() -> ({ v: Q | v.x.n <= v.y }, u32) { (Q { x: C { n: 0 }, y: 10 }, 0) }

fn main() with {} {
    let mut t = make();
    let mut c = || t.0.x.set(50); // error: can't borrow `t.0.x` as `&mut`: it's in `t.0`, whose type has a refinement
    c();
    need(&t.0);
}
