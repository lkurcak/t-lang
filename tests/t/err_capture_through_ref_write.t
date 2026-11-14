// A closure reading `r.x` through `r: &mut S` borrows `(*r).x`: it can't
// be changed through `r` while the closure lives, though `r.y` can.
#[derive(Discard)]
struct S { x: u64, y: u64 }

fn f(r: &mut S) -> u64 {
    let c = || r.x;
    r.y = 2;
    r.x = 3; // error: cannot assign to `r.x` while
    c()
}

fn main() {
    let mut s = S { x: 1, y: 1 };
    let _ = f(&mut s);
}
