// An impl that writes its own body for a method with a default is checked
// against the law as any impl's method is (docs/88-iterator-laws.md).
trait Steps {
    fn step(&self) -> usize;

    fn twice(&self) -> usize with panic { self.step() * 2 }
}

trait Even: Steps {
    law fn twice(&self) -> { t: usize | t % 2 == 0 } with panic;
}

#[derive(Discard)]
struct A {}

impl Steps for A {
    fn step(&self) -> usize { 3 }

    fn twice(&self) -> usize { 7 }
}

impl Even for A {}

fn main() {}
// error: can't prove `t % 2 == 0` for the result of `twice`
