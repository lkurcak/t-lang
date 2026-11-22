// An impl's own method, in place of a default that says `with panic
// unless Self: Small`, is what a call through the trait runs: of a type
// that's `Small` it can't panic either (lemma B6 of
// docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

#[derive(Discard)]
struct B { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

trait Bump {
    fn bump(&self) -> u8 with panic unless Self: Small { 1 }
}

impl Bump for A {
    fn bump(&self) -> u8 with panic { self.n + 250 }
}

fn main() with {} {
    let _v = (A { n: 9 }).bump();
}
// error: `+` may panic where `A: Small`, but `bump` says `with panic unless A: Small`
