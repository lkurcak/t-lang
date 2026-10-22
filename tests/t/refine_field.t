// A field's refinement is kept by every value of the struct: each literal
// must have it, and every read of the field knows it.
#[derive(Copy, Discard)]
struct Span {
    lo: { l: i64 | l >= 0 },
    hi: { h: i64 | h >= lo },
}

impl Span {
    fn new(lo: { l: i64 | l >= 0 }, len: { n: i64 | n >= 0 && n <= 1000 }) -> Span with panic {
        Span { lo, hi: lo + len }
    }

    // Can't overflow: `0 <= lo <= hi`.
    fn len(&self) -> i64 { self.hi - self.lo }

    // `len` promises nothing of its result, so `+ 1` may overflow here.
    fn per(&self, total: i64) -> i64 with panic { total / (self.len() + 1) }
}

fn width(s: Span) -> { w: i64 | w >= 0 } { s.hi - s.lo }

fn main() with panic {
    let s = Span { lo: 3, hi: 10 };
    print(s.len()); // expect: 7
    print(width(s)); // expect: 7
    let t = Span::new(5, 2);
    print(t.hi); // expect: 7
    print(t.per(9)); // expect: 3
}
