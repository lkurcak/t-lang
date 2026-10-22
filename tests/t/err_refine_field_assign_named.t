// Assigning a field that another field's refinement names must keep that
// refinement too.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn main() {
    let mut s = Span { lo: 1, hi: 3 };
    s.lo = 4;
    print(s.lo);
}
// error: can't prove `h >= lo` for field `hi` of `Span`, which names `lo`
