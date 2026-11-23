// An impl's associated effect is of no call, so it can't have a
// condition on what one is given; it can say `unless` a bound holds
// (docs/96-associated-effects-unless.md).
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {}

#[derive(Discard)]
struct Lo { n: u8 }

impl Val for Lo {
    fn value(&self) -> u8 { self.n }
}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic if true;

    fn go(&self) -> u8 with panic { self.x.value() + 1 }
}

fn main() {}
// error: an associated effect can't have a condition: it's of no call, whose parameters it could say; it can say `unless` a bound holds
