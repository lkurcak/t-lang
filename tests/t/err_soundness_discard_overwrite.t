// Overwriting `p` drops the old value, whose `discard` reads `x`, gone by
// then.
struct Peek<'a> { r: &'a i64 }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _r = *self.r;
    }
}

fn main() {
    let y = 1;
    let mut p;
    {
        let x = 5;
        p = Peek { r: &x };
    }
    p = Peek { r: &y };
}
// error: `x` doesn't live long enough
