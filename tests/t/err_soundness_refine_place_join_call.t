// After an `if` whose branch lends the struct to a call, its field is
// whatever the call left, or what it was.
#[derive(Copy, Discard)]
struct P { x: int }

fn zero(p: &mut P) { p.x = 0; }

fn f(mut p: P, c: bool) -> int {
    if p.x > 0 {
        if c {
            zero(&mut p);
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
