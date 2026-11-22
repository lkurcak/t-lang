// A function used as a value has every effect it says, whatever bound
// holds: what calls it knows only its type (docs/92-where-a-bound-holds.md).
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

#[allow(unused_effects)]
fn idle<T: Discard>(x: T) -> u8 with panic unless T: Small {
    x.discard();
    1
}

fn main() with {} {
    let f: fn(A) -> u8 = idle::<A>;
    let _v = f(A { n: 1 });
}
// error: expected `fn(A) -> u8`, found `fn(A) -> u8 with panic`
