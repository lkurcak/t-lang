// An impl's method that spells its associated effect out, `with panic
// unless T: Low`, is checked again with `T: Low`, where it may not panic
// (lemma B4 of docs/92-where-a-bound-holds.md, A5 of
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

    fn go(&self) -> u8 with panic unless T: Low { self.x.value() + 2 }
}

fn main() with {} {
    let _v = (W { x: Lo { n: 0 } }).go();
}
// error: `+` may panic where `T: Low`, but `go` says `with panic unless T: Low`
