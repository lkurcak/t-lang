// Nor to a field another's refinement names.
#[derive(Copy, Discard)]
struct Span { lo: int, hi: { h: int | h >= lo } }

fn big(r: &mut int) { *r = 100; }

fn main() {
    let mut s = Span { lo: 1, hi: 3 };
    big(&mut s.lo);
    print(s.lo);
}
// error: can't borrow `s.lo` as `&mut`: the refinement of field `hi` of `Span` names it, which a write through the reference could break
