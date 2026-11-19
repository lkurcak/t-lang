// A trait's default body is checked against what the trait says of it
// (docs/86-trait-contracts.md).
trait Get {
    fn get(&self) -> u8;

    fn small(&self) -> { v: u8 | v < 10 } { self.get() }
}

fn main() {}
// error: can't prove `v < 10` for the result of `small`
