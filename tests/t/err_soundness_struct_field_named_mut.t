// A field another field's refinement names can't be borrowed `&mut` but
// straight to a call: the reference couldn't say what `hi >= lo` needs of
// what's written through it.
#[derive(Copy, Discard)]
struct Span { lo: u8, hi: { h: u8 | h >= lo } }

fn main() {
    let mut s = Span { lo: 1, hi: 3 };
    let r = &mut s.lo;
    *r = 100;
    let _ = s.hi;
}
// error: can't borrow `s.lo` as `&mut`: the refinement of field `hi` of `Span` names it
