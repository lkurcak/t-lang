// So can one that captures a `&mut` to it, which isn't `mut` itself.
#[derive(Copy, Discard)]
struct P { x: int }

fn f(r: &mut P) -> int {
    if r.x > 0 {
        let mut c = || r.x = 0;
        c();
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
