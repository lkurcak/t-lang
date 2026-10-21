// A call whose result can hold the `&mut` it's lent lends it for longer
// than the call: writes through the result change the place.
#[derive(Copy, Discard)]
struct P { x: int }

fn field(p: &mut P) -> &mut int { &mut p.x }

fn f(mut p: P) -> int {
    if p.x > 0 {
        let r = field(&mut p);
        *r = 0;
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
