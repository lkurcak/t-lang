// A `&'a [&'b i64]` is only a type if `'b` outlives `'a`, as for a `&'a
// &'b i64`, so a function given one may count on it.
fn first<'a, 'b>(s: &'a [&'b i64]) -> Option<&'a i64> {
    match s.first() {
        Option::Some(r) => Option::Some(*r),
        Option::None => Option::None,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&first(core::slice::empty()).is_none())?; // expect: true
    Result::Ok(())
}
