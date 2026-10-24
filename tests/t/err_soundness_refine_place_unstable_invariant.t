// What changes other than by assignment still has its struct's
// refinements, but each read can be a different value.
#[derive(Copy, Discard)]
struct Span { lo: i64, hi: { h: i64 | h >= lo } }

fn shift(s: &mut Span) { *s = Span { lo: 0, hi: 0 }; }

fn f(mut s: Span) -> i64 {
    if s.lo > 0 {
        shift(&mut s);
        10 / s.hi
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(Span { lo: 1, hi: 2 }))?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
