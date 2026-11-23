// An impl's method of a trait has at most its trait's method's effects, as
// each copy has them: where the impl's associated effect has `panic` only
// unless `T: Low`, the method may panic only where `T` isn't `Low`, which
// it must say (lemma A5 of docs/96-associated-effects-unless.md).
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic { self.x.value() + 1 }
}

fn main() {}
// error: `go` may panic where `T: Low`, but `Go`'s `go` can't there, as this impl's associated effect says; so it can't either: say `with panic unless T: Low`
