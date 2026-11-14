// While a closure borrows `(*r).x`, all of `*r` can't be lent `&mut`.
#[derive(Discard)]
struct S { x: u64, y: u64 }

fn reset(s: &mut S) {
    s.x = 0;
}

fn f(r: &mut S) -> u64 {
    let c = || r.x;
    reset(r); // error: cannot borrow
    c()
}

fn main() {
    let mut s = S { x: 1, y: 1 };
    let _ = f(&mut s);
}
