// A `&mut self` method called on `self` can change `self`'s fields.
#[derive(Copy, Discard)]
struct P { x: i64 }

impl P {
    fn clear(&mut self) { self.x = 0; }

    fn f(&mut self) -> i64 {
        if self.x > 0 {
            self.clear();
            10 / self.x
        } else {
            0
        }
    }
}

fn main() {
    let mut p = P { x: 1 };
    print(p.f());
}
// error: `/` may panic, but `f` doesn't say `with panic`
