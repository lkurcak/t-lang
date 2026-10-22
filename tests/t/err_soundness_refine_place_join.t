// After an `if` whose branch assigns the field, it's either value.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn f(mut p: P, c: bool) -> i64 {
    if p.x > 0 {
        if c {
            p.x = 0;
        }
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }, true));
}
// error: `/` may panic, but `f` doesn't say `with panic`
