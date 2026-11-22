// A clause's bound is on a type parameter too
// (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f(n: u8) {
    let mut i = 0u8;
    while i < n invariant i <= n where A: Small {
        i = i + 1;
    }
}

fn main() {}
// error: a bound here can only be on a type parameter, `Self`, or an associated type of one, not `A`
