// What an impl's associated effect hangs on can be a bound of an
// associated type of its parameter, `I::Item: Low`: decided where `I` is
// known, by what `I::Item` is there (lemma A1 of
// docs/96-associated-effects-unless.md).
trait Low {}

impl Low for u8 {}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where I: Discard)]
struct W<I> { it: I, n: u8 }

impl<I: Iterator> Go for W<I> {
    effect e = panic unless I::Item: Low;

    fn go(&self) -> u8 with Self::e { self.n }
}

#[derive(Discard)]
struct Up { n: u8 }

impl Iterator for Up {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> { None }
}

#[derive(Discard)]
struct Signed { n: i8 }

impl Iterator for Signed {
    type Item = i8;
    effect e = {};

    fn next(&mut self) -> Option<i8> { None }
}

/// Of `Up`, whose items are `u8`s, which are `Low`: no `panic`.
fn low() -> u8 with {} { (W { it: Up { n: 0 }, n: 3 }).go() }

/// Of `Signed`, whose aren't: it may.
fn signed() -> u8 with panic { (W { it: Signed { n: 0 }, n: 4 }).go() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&low())?;    // expect: 3
    out.line(&signed())?; // expect: 4
    Ok(())
}
