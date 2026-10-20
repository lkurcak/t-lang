// Generic code knows what a bound decides from its own bounds: `Map<I, F>`
// gives `U`s, as `F: FnMut(I::Item) -> U` says.
use core::iter::Map;

fn first<I: Iterator + Discard, U: Discard, f, F: FnMut(I::Item) -> U with f + Discard>(
    mut m: Map<I, F>,
) -> Option<U> with {I::e, f} { m.next() }

fn main() with panic {
    match first((3..10).map(|x| x * x)) {
        Option::Some(n) => print(n), // expect: 9
        Option::None => {},
    }
    let total = (1..4).map(|x| x * 2).filter(|x| *x > 2).fold(0, |a, b| a + b);
    print(total); // expect: 10
}
