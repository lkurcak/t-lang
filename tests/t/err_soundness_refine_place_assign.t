// Assigning a field gives it a new value: what was known of the old one
// isn't of it.
#[derive(Copy, Discard)]
struct P { x: int }

fn f(mut p: P) -> int {
    if p.x > 0 {
        p.x = 0;
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
