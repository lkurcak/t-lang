// The refinements of a literal are checked whatever order its fields are
// written in, with each field's own value.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = Span { hi: 1, lo: 5 };
    out.line(&s.hi)?;
    Result::Ok(())
}
// error: can't prove `h >= lo` for field `hi` of `Span`
