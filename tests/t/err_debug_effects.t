// `Debug::fmt` does only what its sink does: an impl can't panic.
#[derive(Discard)]
struct P { n: u8 }

impl core::fmt::Debug for P {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        if self.n == 0 {
            panic("zero");
        }
        w.write_str("P")
    }
}

fn main() {}
// error: `panic` may panic, but `fmt` doesn't say `with panic`
