// A closure that captures the struct can change its fields when it's
// called.
#[derive(Copy, Discard)]
struct P { x: int }

fn f(mut p: P) -> int {
    if p.x > 0 {
        let mut c = || p.x = 0;
        c();
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
