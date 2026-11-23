// An iterator whose `e` must be within `<W<T> as Go>::e` can't be one
// that panics: of a `T` that's `Low`, that's none (lemma A2 of
// docs/96-associated-effects-unless.md).
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Lo { n: u8 }

#[derive(Discard)]
struct Hi { n: u8 }

impl Val for Lo {
    fn value(&self) -> u8 { 254 }
}

impl Low for Lo {}

impl Val for Hi {
    fn value(&self) -> u8 { 3 }
}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

fn boom() -> u8 with panic { panic("boom") }

#[derive(Discard)]
struct Down { n: u8 }

impl Iterator for Down {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic { panic("down") }
}

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic unless T: Low { self.x.value() + 1 }
}

fn first<T: Val, J: Iterator<Item = u8, e = <W<T> as Go>::e> + Discard>(
    _w: &W<T>,
    mut j: J,
) -> Option<u8> with <W<T> as Go>::e {
    let r = j.next();
    j.discard();
    r
}

fn outer<T: Val>(w: &W<T>) -> Option<u8> with <W<T> as Go>::e { first(w, Down { n: 1 }) }

fn main() with {} {
    let _v = outer(&W { x: Lo { n: 0 } });
}
// error: `<Down as Iterator>::e` may panic, but `first` needs `J::e = <W<T> as Go>::e`
