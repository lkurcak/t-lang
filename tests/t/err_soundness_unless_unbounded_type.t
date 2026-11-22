// Nor of a type that doesn't meet it, where an impl's own method may
// panic (docs/92-where-a-bound-holds.md).
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
    fn bump(&self) -> u8 with panic unless Self: Small { 1 }
}

impl Bump for B {
    fn bump(&self) -> u8 with panic { self.n + 250 }
}

fn main() with {} {
    let _v = (B { n: 9 }).bump();
}
// error: `bump` may panic, but `main` doesn't say `with panic`
