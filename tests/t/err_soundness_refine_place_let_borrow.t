// A `&mut` kept in a variable lets later writes through it change the
// field.
#[derive(Copy, Discard)]
struct P { x: int }

fn f(mut p: P) -> int {
    if p.x > 0 {
        let r = &mut p;
        r.x = 0;
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
