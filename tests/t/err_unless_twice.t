// An effect is said once (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f<T>(_x: &T) with {panic, panic unless T: Small} {}

fn main() {}
// error: `panic` is said twice: say it once
