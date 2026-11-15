// A struct's field whose type is a type argument is as a tuple's element:
// `Pair<{ v: Q | v.x <= v.y }>`'s `a` has a refinement relating `a.x` to
// `a.y`, which `p`'s type says, though no refinement is written at `p`. A
// closure changing `p.a.x` captures `p.a` (K2a of
// docs/79-what-a-capture-splits.md); in a nested struct too, `p.a.u`.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

#[derive(Discard)]
struct Pair<T> { a: T, n: u32 }

#[derive(Discard)]
struct W<U> { u: U, k: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn make() -> Pair<{ v: Q | v.x <= v.y }> { Pair { a: Q { x: 0, y: 10 }, n: 0 } }

fn nested() -> Pair<W<{ v: Q | v.x <= v.y }>> {
    Pair { a: W { u: Q { x: 0, y: 10 }, k: 0 }, n: 0 }
}

fn main() with {} {
    let mut p = make();
    let mut c = || p.a.x = 50; // error: can't prove `v.x <= v.y` for `p.a`, once `p.a.x` is written
    c();
    need(&p.a);
    let mut w = nested();
    let mut d = || w.a.u.x = 50; // error: can't prove `v.x <= v.y` for `w.a.u`, once `w.a.u.x` is written
    d();
    need(&w.a.u);
}
