// Through a reference, the same: the other fields are what's read through
// it.
#[derive(Copy, Discard)]
struct Span { lo: int, hi: { h: int | h >= lo } }

fn set(s: &mut Span, h: int) { s.hi = h; }

fn main() {
    let mut s = Span { lo: 1, hi: 3 };
    set(&mut s, 0);
    print(s.hi);
}
// error: can't prove `h >= lo` for field `hi` of `Span`
