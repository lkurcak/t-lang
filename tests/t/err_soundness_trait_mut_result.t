// What a trait's method promises names its parameters' values as passed:
// an impl's can't make one it names `mut`, which could be another by
// return (docs/86-trait-contracts.md). Here `k` would be 100.
trait Tr {
    fn f(&self, k: usize) -> { n: usize | n <= k };
}

#[derive(Discard)]
struct X {}

impl Tr for X {
    fn f(&self, mut k: usize) -> usize {
        k = 100;
        k
    }
}

fn g<T: Tr>(t: &T) -> u8 {
    let a = [7u8; 2];
    a[t.f(1)]
}

fn main() with {} {
    let _ = g(&X {});
}
// error: `k` can't be `mut` in `f`: what trait `Tr` promises names it, as it was passed; write `let mut k = k;` in the body
