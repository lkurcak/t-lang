// Passing on a `&mut` lets the callee write through it.
#[derive(Copy, Discard)]
struct P { x: int }

fn zero(p: &mut P) { p.x = 0; }

fn f(r: &mut P) -> int {
    if r.x > 0 {
        zero(r);
        10 / r.x
    } else {
        0
    }
}

fn main() with panic {
    let mut p = P { x: 1 };
    print(f(&mut p));
}
// error: `/` may panic, but `f` doesn't say `with panic`
