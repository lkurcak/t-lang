// So can a `&mut` to the field.
#[derive(Copy, Discard)]
struct P { x: int }

fn zero(r: &mut int) { *r = 0; }

fn f(mut p: P) -> int {
    if p.x > 0 {
        zero(&mut p.x);
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
