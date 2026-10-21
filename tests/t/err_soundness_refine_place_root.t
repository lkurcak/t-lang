// A reference that's assigned points somewhere else: what was known
// through it isn't any more.
#[derive(Copy, Discard)]
struct P { x: int }

fn f(a: &P, b: &P) -> int {
    let mut r = a;
    if r.x > 0 {
        r = b;
        10 / r.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(&P { x: 1 }, &P { x: 0 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
