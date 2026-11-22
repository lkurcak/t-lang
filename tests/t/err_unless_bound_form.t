// The bound is a trait alone (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f<T>(_x: &T) with panic unless T: Iterator<Item = u8> {}

fn main() {}
// error: expected a trait alone here: `T: Trait`, with nothing said of what's in it
