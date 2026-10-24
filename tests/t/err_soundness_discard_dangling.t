// A `discard` of its own may read what its value borrows, so dropping one
// uses it: `x` would be gone when `p`'s `discard` reads it.
struct Peek<'a> { r: &'a i64, out: &'a mut std::io::Stdout }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _ = self.out.line(self.r);
    }
}

fn main(os: std::Os) {
    let mut out = os.stdout;
    let _p;
    {
        let x = 5;
        _p = Peek { r: &x, out: &mut out };
    }
}
// error: `x` doesn't live long enough
