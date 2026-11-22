// A closure's effects are what they are whatever bound holds: a call of
// one that may panic may, where the function says it can't
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

fn bump<T: Discard>(x: T, n: u8) -> u8 with panic unless T: Small {
    x.discard();
    let f = |m: u8| m + 250;
    f(n)
}

fn main() with {} {
    let _v = bump(A { n: 1 }, 9);
}
// error: `f` may panic where `T: Small`, but `bump` says `with panic unless T: Small`
