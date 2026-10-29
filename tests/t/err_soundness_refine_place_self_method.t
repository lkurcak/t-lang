// A `&mut self` method called on `self` can change `self`'s fields.
#[derive(Copy, Discard)]
struct P { x: i64 }

impl P {
    fn clear(&mut self) { self.x = 0; }

    fn f(&mut self) -> i64 with {} {
        if self.x > 0 {
            self.clear();
            10 / self.x
        } else {
            0
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = P { x: 1 };
    out.line(&p.f())?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
