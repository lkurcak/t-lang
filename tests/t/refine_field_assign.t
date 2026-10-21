// Assigning a field must keep every refinement that's about it: its own,
// and those of later fields that name it.
#[derive(Copy, Discard)]
struct Span {
    lo: { l: int | l >= 0 },
    hi: { h: int | h >= lo },
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

fn main() {
    let mut s = Span { lo: 1, hi: 2 };
    s.shrink();
    s.shrink();
    print(s.lo); // expect: 2
    s.grow();
    s.widen();
    print(s.lo); // expect: 1
    print(s.hi); // expect: 3
    s.hi = s.lo;
    print(s.hi); // expect: 1
}
