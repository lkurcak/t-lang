// A literal must have its struct's refinements.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = Span { lo: 5, hi: 3 };
    out.line(&s.hi)?;
    Result::Ok(())
}
// error: can't prove `h >= lo` for field `hi` of `Span`
