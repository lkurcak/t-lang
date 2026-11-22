// Nor may a `panic(..)` be reached where the bound holds
// (docs/92-where-a-bound-holds.md).
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

fn safe<T: Discard>(x: T, n: u8) -> u8 with panic unless T: Small {
    x.discard();
    if n > 5 {
        panic("big");
    }
    n
}

fn main() with {} {
    let _v = safe(A { n: 1 }, 9);
}
// error: `panic` may panic where `T: Small`, but `safe` says `with panic unless T: Small`
