// Assigning a refined field must keep its refinement.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = Span { lo: 1, hi: 3 };
    s.hi = 0;
    out.line(&s.hi)?;
    Result::Ok(())
}
// error: can't prove `h >= lo` for field `hi` of `Span`
