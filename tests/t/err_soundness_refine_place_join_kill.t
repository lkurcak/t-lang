// After an `if` whose branch assigns the whole struct, its fields are
// either value, though that branch never read them.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P, q: P, c: bool) -> i64 {
    if p.x > 0 {
        if c {
            p = q;
        }
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }, P { x: 0 }, true));
}
// error: `/` may panic, but `f` doesn't say `with panic`
