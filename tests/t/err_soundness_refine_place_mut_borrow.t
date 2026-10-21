// A `&mut` to the struct can change its fields.
#[derive(Copy, Discard)]
struct P { x: int }

fn zero(p: &mut P) { p.x = 0; }

fn f(mut p: P) -> int {
    if p.x > 0 {
        zero(&mut p);
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
