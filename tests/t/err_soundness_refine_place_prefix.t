// Assigning the whole struct changes its fields.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P, q: P) -> i64 {
    if p.x > 0 {
        p = q;
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }, P { x: 0 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
