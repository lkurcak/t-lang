// Of a type that isn't `Low`, and can't be, `<W<Hi> as Go>::e` is `panic`:
// decided, for every copy (lemma A1 of docs/96-associated-effects-unless.md).
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Hi { n: u8 }

impl Val for Hi {
    fn value(&self) -> u8 { 255 }
}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with Self::e { self.x.value() + 1 }
}

fn main() with {} {
    let _v = (W { x: Hi { n: 0 } }).go();
}
// error: `go` may panic, but `main` doesn't say `with panic`
