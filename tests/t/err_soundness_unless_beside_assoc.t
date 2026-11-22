// An `unless` takes out only the effect it's on: an associated effect
// beside it that's `panic` at a call is still there (lemma B5 of
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

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;

    fn twice(&self) -> u8 with {Self::e, panic unless Self: Small} { self.go() }
}

impl Go for A {
    effect e = panic;

    fn go(&self) -> u8 with panic { self.n + 250 }
}

fn main() with {} {
    let _v = (A { n: 9 }).twice();
}
// error: `twice` may panic, but `main` doesn't say `with panic`
