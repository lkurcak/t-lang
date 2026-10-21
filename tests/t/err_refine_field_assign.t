// Assigning a refined field must keep its refinement.
#[derive(Copy, Discard)]
struct Span { lo: int, hi: { h: int | h >= lo } }

fn main() {
    let mut s = Span { lo: 1, hi: 3 };
    s.hi = 0;
    print(s.hi);
}
// error: can't prove `h >= lo` for field `hi` of `Span`
