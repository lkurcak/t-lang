// A `&'a [&'b i64; 1]` is only a type if `'b` outlives `'a`, as for a `&'a
// &'b i64`, so a function given one may count on it.
fn first<'a, 'b>(a: &'a [&'b i64; 1]) -> &'a i64 { a[0] }

/// An array of references is covariant in their lifetime, as they are:
/// a `Pair<'b>` is a `Pair<'a>` if `'b` outlives `'a`.
#[derive(Copy, Discard)]
struct Pair<'a> { refs: [&'a i64; 2] }

fn shorten<'a, 'b: 'a>(p: Pair<'b>) -> Pair<'a> { p }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 5;
    out.line(first(&[&n]))?;          // expect: 5
    let x: i64 = 7;
    let y: i64 = 8;
    let p = shorten(Pair { refs: [&x, &y] });
    out.line(p.refs[1])?;             // expect: 8
    Result::Ok(())
}
