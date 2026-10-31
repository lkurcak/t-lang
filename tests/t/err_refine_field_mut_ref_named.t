// Nor to a field another's refinement names: `big` breaks `hi >= lo`.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn big(r: &mut i64) {
    *r = 100;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = Span { lo: 1, hi: 3 };
    big(&mut s.lo);
    out.line(&s.lo)?;
    Result::Ok(())
}
// error: can't prove `h >= lo` for field `hi` of `Span`, which names `lo`
