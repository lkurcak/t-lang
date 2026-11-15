// A closure's parameter given a tuple whose element has a refinement: a
// closure in its body changing `t.0.x` captures `t` (K2b of
// docs/79-what-a-capture-splits.md), as it would `t.0` by K2a.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn h<T, F: FnOnce(T)>(x: T, f: F) { f(x) }

fn main() with {} {
    let q: { v: Q | v.x <= v.y } = Q { x: 0, y: 10 };
    h((q, 0u32), |mut t| {
        let mut c = || t.0.x = 50; // error: can't prove what's declared (one of the values the call has of its type) for `t.0`, once `t.0.x` is written
        c();
        need(&t.0);
    });
}
