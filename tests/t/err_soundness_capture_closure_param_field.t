// A closure's parameter is declared what the call it's given to gives it,
// which only the refinement checker knows: a closure in its body changing
// `t.x` captures `t` whole (K2b of docs/79-what-a-capture-splits.md), so
// its write is checked against what's declared of `t`. From 0.91.0 to
// 0.93.0 it captured `t.x` alone, and `need` read a `t` whose `x` was more
// than its `y`.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn h<T, F: FnOnce(T)>(x: T, f: F) { f(x) }

fn main() with {} {
    let q: { v: Q | v.x <= v.y } = Q { x: 0, y: 10 };
    h(q, |mut t| {
        let mut c = || t.x = 50; // error: can't prove what's declared (one of the values the call has of its type) for `t`, once `t.x` is written
        c();
        need(&t);
    });
}
