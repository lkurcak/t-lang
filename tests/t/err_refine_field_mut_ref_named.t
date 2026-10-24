// Nor to a field another's refinement names.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn big(r: &mut i64) { *r = 100; }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = Span { lo: 1, hi: 3 };
    big(&mut s.lo);
    out.line(&s.lo)?;
    Result::Ok(())
}
// error: can't borrow `s.lo` as `&mut`: the refinement of field `hi` of `Span` names it, which a write through the reference could break
