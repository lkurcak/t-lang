// Variables go out of scope in the reverse of their order, so `x` goes
// before `p`, whose `discard` would read it.
struct Peek<'a> { r: &'a i64, out: &'a mut std::io::Stdout }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _ = self.out.line(self.r);
    }
}

fn main(os: std::Os) {
    let mut out = os.stdout;
    let zero = 0;
    let mut p = Peek { r: &zero, out: &mut out };
    let x = 5;
    p.r = &x;
}
// error: `x` doesn't live long enough
