// A field with a refinement of its own is captured whole: its refinement
// relates the places in it, `q.a` to `q.b`. Its siblings are apart.
#[derive(Discard)]
struct Q { a: u32, b: u32 }

#[derive(Discard)]
struct P { q: { q: Q | q.a < q.b }, n: u32 }

fn main() {
    let mut p = P { q: Q { a: 0, b: 10 }, n: 0 };
    let mut c = || p.q.a = 5;
    p.n = 1;
    p.q.b = 3; // error: the closure captures `p.q`, not just `p.q.a`, as field `q` of `P` has a refinement
    c();
}
