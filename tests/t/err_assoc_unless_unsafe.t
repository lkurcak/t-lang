// Only `panic`, `alloc` and `global` can hang on a bound in an impl's
// associated effect, as in a function's (docs/96-associated-effects-unless.md).
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
    effect e = unsafe unless T: Low;

    fn go(&self) -> u8 with unsafe { self.x.value() + 1 }
}

fn main() {}
// error: only `panic`, `alloc` and `global` can say `unless`: `unsafe` says what the caller must keep, and an effect parameter or an associated effect is what's given
