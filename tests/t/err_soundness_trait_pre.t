// A call through a bound proves what the trait's method asks
// (docs/86-trait-contracts.md).
trait Pick {
    fn pick(&self, i: { i: usize | i < 4 }) -> u8;
}

fn fifth<P: Pick>(p: &P) -> u8 { p.pick(4) }

fn main() {}
// error: can't prove `i < 4` for argument `i` of `pick`
