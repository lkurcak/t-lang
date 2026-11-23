// What an atom of an impl's associated effect hangs on is a bound of a
// type parameter of the impl's, or of an associated type of one: of `Lo`,
// it either holds or doesn't (docs/96-associated-effects-unless.md).
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
    effect e = panic unless Lo: Low;

    fn go(&self) -> u8 with panic { self.x.value() + 1 }
}

fn main() {}
// error: a bound here can only be on a type parameter, `Self`, or an associated type of one, not `Lo`, which either meets it or doesn't
