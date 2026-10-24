// Assigning a field must keep every refinement that's about it: its own,
// and those of later fields that name it.
#[derive(Copy, Discard)]
struct Span {
    lo: { l: i64 | l >= 0 },
    hi: { h: i64 | h >= lo },
}

impl Span {
    fn shrink(&mut self) {
        if self.lo < self.hi {
            self.lo = self.lo + 1;
        }
    }

    fn grow(&mut self) {
        if self.hi < 100 {
            self.hi = self.hi + 1;
        }
    }

    fn widen(&mut self) {
        if self.lo > 0 {
            self.lo = self.lo - 1;
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = Span { lo: 1, hi: 2 };
    s.shrink();
    s.shrink();
    out.line(&s.lo)?; // expect: 2
    s.grow();
    s.widen();
    out.line(&s.lo)?; // expect: 1
    out.line(&s.hi)?; // expect: 3
    s.hi = s.lo;
    out.line(&s.hi)?; // expect: 1
    Result::Ok(())
}
