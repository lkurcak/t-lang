// An atom of a binding hangs on its bound only as the binding says it:
// `<Enumerate<W<T>> as Iterator>::e` has `panic` always, as `W<T>`'s `e`
// is `panic`, whether `W<T>` is bounded or not. Only `Enumerate`'s own
// `panic` hangs on it (lemma A4 of docs/96-associated-effects-unless.md).
use core::iter::Enumerate;

trait Foo {}

#[derive(Discard)]
struct A { n: u8 }

impl Foo for A {}

#[derive(Discard where T: Discard)]
struct W<T> { t: T, left: bool }

impl<T> Iterator for W<T> {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic {
        if self.left {
            self.left = false;
            return Some(7);
        }
        None
    }
}

/// Bounded, of a `T` that's `Foo`: so whether `Enumerate`'s `panic` is
/// there isn't decided in code generic in `T`.
impl<T: Foo> BoundedIterator for W<T> {
    #[measure]
    fn most(&self) -> usize {
        if self.left { 1 } else { 0 }
    }
}

fn count<T: Discard>(w: W<T>) -> usize with <Enumerate<W<T>> as Iterator>::e {
    let mut n = 0;
    for (i, _x) in w.enumerate() {
        n = i + 1;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&count(W { t: A { n: 0 }, left: true }))?; // expect: 1
    Ok(())
}
