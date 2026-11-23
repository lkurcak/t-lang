// An atom is said once in an impl's associated effect: with `unless` or
// without (docs/96-associated-effects-unless.md).
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
    effect e = {panic, panic unless T: Low};

    fn go(&self) -> u8 with panic { self.x.value() + 1 }
}

fn main() {}
// error: `panic` is said twice: say it once
