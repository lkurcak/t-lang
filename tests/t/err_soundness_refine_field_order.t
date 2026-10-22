// The refinements of a literal are checked whatever order its fields are
// written in, with each field's own value.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn main() {
    let s = Span { hi: 1, lo: 5 };
    print(s.hi);
}
// error: can't prove `h >= lo` for field `hi` of `Span`
