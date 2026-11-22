// A call has the effect wherever the bound isn't known to hold: of a type
// parameter that may be any type (docs/92-where-a-bound-holds.md).
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

fn call<T: Discard>(x: T) -> u8 with {} { idle(x) }

fn main() with {} {
    let _v = call(B { n: 1 });
}
// error: `idle` may panic, but `call` doesn't say `with panic`
