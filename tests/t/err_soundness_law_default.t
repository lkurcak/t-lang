// A law of a method with a default body: the default must keep it, for
// every type of the law's trait (docs/88-iterator-laws.md).
trait Steps {
    fn step(&self) -> usize;

    fn twice(&self) -> usize with panic { self.step() + self.step() + 1 }
}

trait Even: Steps {
    law fn twice(&self) -> { t: usize | t % 2 == 0 } with panic;
}

fn main() {}
// error: can't prove `t % 2 == 0` for the result of `twice`
