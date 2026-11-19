// An impl can't ask more than its trait: a call through the trait proves
// only what the trait asks (docs/86-trait-contracts.md).
trait Pick {
    fn pick(&self, i: { i: usize | i < 4 }) -> u8;
}

#[derive(Discard)]
struct Two { a: [u8; 2] }

impl Pick for Two {
    fn pick(&self, i: { i: usize | i < 2 }) -> u8 { self.a[i] }
}

fn third<P: Pick>(p: &P) -> u8 { p.pick(3) }

fn main() { let _ = third(&Two { a: [1, 2] }); }
// error: can't prove `i < 2` from what trait `Pick` asks of `i`
