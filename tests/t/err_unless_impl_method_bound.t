// An impl's method may say `unless` a bound, but only of a bound its
// trait's method's effects hang on there: `T: Other` holding doesn't make
// the impl's associated effect lose its `panic`, which only `T: Low` does
// (lemma A5 of docs/96-associated-effects-unless.md).
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {}

trait Other {}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic unless T: Other { self.x.value() + 1 }
}

fn main() {}
// error: `go` may panic where `T: Low`, but `Go`'s `go` can't there, as this impl's associated effect says; so it can't either: say `with panic unless T: Low`
