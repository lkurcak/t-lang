// A variable declared with a refinement is captured whole when a closure
// changes part of it: its refinement can relate that part to the rest.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn main() {
    let mut p: { q: P | q.x < q.y } = P { x: 0, y: 10 };
    let mut c = || p.x = 5;
    p.y = 3; // error: cannot assign to `p.y` while `p` is mutably borrowed
    c();
}
