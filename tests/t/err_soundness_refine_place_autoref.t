// A method taking `&mut self` borrows the struct mutably, without a
// `&mut` written.
#[derive(Copy, Discard)]
struct P { x: int }

impl P {
    fn clear(&mut self) { self.x = 0; }
}

fn f(mut p: P) -> int {
    if p.x > 0 {
        p.clear();
        10 / p.x
    } else {
        0
    }
}

fn main() with panic {
    print(f(P { x: 1 }));
}
// error: `/` may panic, but `f` doesn't say `with panic`
