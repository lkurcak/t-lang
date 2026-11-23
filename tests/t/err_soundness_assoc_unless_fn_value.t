// `<W<T> as Go>::e`, of a `T` that may or may not be `Low`, is itself: in
// a set that must be within it, it's never taken to be its binding with
// `panic`, which it isn't where `T: Low`. So a function that panics isn't
// a `fn() -> u8 with <W<T> as Go>::e` (lemma A2 of
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

fn pick<T: Val>(_w: &W<T>) -> fn() -> u8 with <W<T> as Go>::e { boom }

fn main() with {} {
    let w = W { x: Lo { n: 0 } };
    let f = pick(&w);
    let _v = f();
}
// error: expected `fn() -> u8 with <W<T> as Go>::e`, found `fn() -> u8 with panic`
