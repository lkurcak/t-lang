// A function saying `with panic unless T: Small` can't panic where `T` is
// `Small`, which a call there counts on (docs/92-where-a-bound-holds.md).
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
    n + 250
}

fn main() with {} {
    let _v = bump(A { n: 1 }, 9);
}
// error: `+` may panic where `T: Small`, but `bump` says `with panic unless T: Small`
