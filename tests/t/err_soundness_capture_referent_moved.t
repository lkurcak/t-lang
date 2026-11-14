// A closure that changes a field through a reference captures the
// reference, whatever variable holds it: what's behind `s` is declared
// `v.x < v.y`, though `s` itself is declared nothing, and were only
// `(*s).x` lent to the closure, `s.y` could be lowered under it.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn f(r: &mut { v: P | v.x < v.y }) {
    let s = r;
    let mut c = || s.x = 5;
    s.y = 3; // error: cannot assign to `s.y` while `s` is mutably borrowed
    c();
}

fn main() {
    let mut p = P { x: 0, y: 10 };
    f(&mut p);
}
