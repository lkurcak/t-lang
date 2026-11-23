// Nor of `Self`, where that's a type only partly a parameter: a bound
// of `W<T>` hangs on `T`'s, which say it (docs/96-associated-effects-unless.md).
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
    effect e = panic unless Self: Low;

    fn go(&self) -> u8 with panic { self.x.value() + 1 }
}

fn main() {}
// error: a bound here can only be on a type parameter, `Self`, or an associated type of one, not `W<T>`, which is only partly one
