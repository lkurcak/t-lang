// A `&mut` to a whole struct is fine: anything written through it is a
// whole value, which keeps the refinements, or a field assignment, which
// is checked.
use core::mem::swap;

#[derive(Copy, Discard)]
struct Span {
    lo: i64,
    hi: { h: i64 | h >= lo },
}

fn set_hi(s: &mut Span, h: i64) {
    if h >= s.lo {
        s.hi = h;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = Span { lo: 1, hi: 2 };
    let mut b = Span { lo: 5, hi: 9 };
    swap(&mut a, &mut b);
    out.line(&a.lo)?; // expect: 5
    set_hi(&mut b, 7);
    out.line(&b.hi)?; // expect: 7
    set_hi(&mut b, 0);
    out.line(&b.hi)?; // expect: 7
    Result::Ok(())
}
