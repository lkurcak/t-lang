// A literal must have its struct's refinements.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn main() {
    let s = Span { lo: 5, hi: 3 };
    print(s.hi);
}
// error: can't prove `h >= lo` for field `hi` of `Span`
