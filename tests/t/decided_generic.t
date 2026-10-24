// Generic code knows what a bound decides from its own bounds: `Map<I, F>`
// gives `U`s, as `F: FnMut(I::Item) -> U` says.
use core::iter::Map;

fn first<I: Iterator + Discard, U: Discard, effect f, F: FnMut(I::Item) -> U with f + Discard>(
    mut m: Map<I, F>,
) -> Option<U> with {I::e, f} { m.next() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    match first((3i64..10).map(|x| x * x)) {
        Option::Some(n) => out.line(&n)?, // expect: 9
        Option::None => {},
    }
    let total = (1..4i64).map(|x| x * 2).filter(|x| *x > 2).fold(0, |a, b| a + b);
    out.line(&total)?; // expect: 10
    Result::Ok(())
}
