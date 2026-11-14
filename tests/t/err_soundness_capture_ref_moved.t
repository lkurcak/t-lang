// While a closure borrows `(*r).x`, the reference `r` it's borrowed
// through can't be lent on, to change it.
#[derive(Discard)]
struct S { x: u64, y: u64 }

fn f(r: &mut S) -> u64 {
    let c = || r.x;
    let r2 = r; // error: cannot borrow `*r` as mutable while `r.x` is borrowed
    r2.x = 5;
    c()
}

fn main() {
    let mut s = S { x: 1, y: 1 };
    let _ = f(&mut s);
}
