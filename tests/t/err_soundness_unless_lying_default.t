// A default method saying `with panic unless Self: Small` may panic only
// of a type that isn't `Small`: its body is walked again with the bound,
// where nothing in it may (lemma B4 of docs/92-where-a-bound-holds.md).
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
    fn get(&self) -> u8;

    fn bump(&self) -> u8 with panic unless Self: Small { self.get() + 250 }
}

impl Bump for A {
    fn get(&self) -> u8 { self.n }
}

fn main() with {} {
    let _v = (A { n: 9 }).bump();
}
// error: `+` may panic where `Self: Small`, but `bump` says `with panic unless Self: Small`
