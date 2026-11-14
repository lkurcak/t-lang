// The field a closure reads can't be changed while it lives.
#[derive(Discard)]
struct P { x: i64, y: i64 }

fn main() {
    let mut p = P { x: 1, y: 2 };
    let a = &mut p.x;
    let c = || p.x; // error: cannot borrow `p.x` while `p.x` is mutably borrowed
    *a = 3;
    let _ = c();
}
