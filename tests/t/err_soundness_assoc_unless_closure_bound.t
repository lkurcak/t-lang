// A closure given where calls of it must fit `fn() -> u8 with <W<T> as
// Go>::e` can't panic: of a `T` that's `Low`, that has no `panic` (lemma
// A2 of docs/96-associated-effects-unless.md).
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

fn apply<T: Val, F: Fn() -> u8 with <W<T> as Go>::e + Discard>(_w: &W<T>, f: F) -> u8
with <W<T> as Go>::e
{ f() }

fn outer<T: Val>(w: &W<T>) -> u8 with <W<T> as Go>::e { apply(w, || boom()) }

fn main() with {} {
    let _v = outer(&W { x: Lo { n: 0 } });
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn() -> u8 with <W<T> as Go>::e`, which may have no `panic`
