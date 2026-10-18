// `c` borrows `b` mutably until its last call, so `b` can't be moved
// before that.
struct B { x: int }

fn take(b: B) -> int {
    let B { x } = b;
    x
}

fn main() {
    let mut b = B { x: 1 };
    let mut c = || b.x = 2;
    let y = take(b);
    c();
    print(y);
}
// error: cannot move out of `b` while `b` is mutably borrowed
