// Only a function's own effects can say `unless`: a function type's are
// of whatever has it (docs/92-where-a-bound-holds.md).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

fn f(_g: fn() with panic unless A: Small) {}

fn main() {}
// error: only a function's own effects can say `unless`, not a type's or a bound's
