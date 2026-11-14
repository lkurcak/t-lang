// A closure that changes what a reference points to captures the
// reference, not the field through it: what's behind it can be refined,
// relating the field to the rest, which the function could change under
// the closure.
#[derive(Discard)]
struct P { x: u32, y: u32 }

fn f(r: &mut { v: P | v.x < v.y }) {
    let mut c = || r.x = 5;
    r.y = 3; // error: cannot assign to `r.y` while `r` is mutably borrowed
    c();
}

fn main() {
    let mut p = P { x: 0, y: 10 };
    f(&mut p);
}
