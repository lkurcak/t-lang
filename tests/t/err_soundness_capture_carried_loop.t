// A closure made in a loop changing a place in a type argument's place:
// captured whole each time (K2a of docs/79-what-a-capture-splits.md).
#[derive(Discard)]
struct Q { x: u32, y: u32 }

#[derive(Discard)]
struct Pair<T> { a: T, n: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn make() -> Pair<{ v: Q | v.x <= v.y }> { Pair { a: Q { x: 0, y: 10 }, n: 0 } }

fn main() with {} {
    let mut p = make();
    let mut i: u32 = 0;
    while i < 2 {
        let mut c = || p.a.x = 50; // error: can't prove `v.x <= v.y` for `p.a`, once `p.a.x` is written
        c();
        i += 1;
    }
    need(&p.a);
}
