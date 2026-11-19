// An impl that asks less than its trait gets less: its body can't count on
// what the trait asks (docs/86-trait-contracts.md).
trait Pick {
    fn pick(&self, i: { i: usize | i < 2 }) -> u8;
}

#[derive(Discard)]
struct Two { a: [u8; 2] }

impl Pick for Two {
    fn pick(&self, i: { i: usize | i < 3 }) -> u8 { self.a[i] }
}

fn main() { let _ = Two { a: [1, 2] }.pick(2); }
// error: can't prove `i < self.len()` for argument `i` of `index`
