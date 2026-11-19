// An impl's method's parameters are its trait's by position, whatever
// their names: one the trait's promise names can't be `mut` as `j` either.
trait Tr {
    fn f(&self, k: usize) -> { n: usize | n <= k };
}

#[derive(Discard)]
struct X {}

impl Tr for X {
    fn f(&self, mut j: usize) -> usize {
        j = 100;
        j
    }
}

fn g<T: Tr>(t: &T) -> u8 {
    let a = [7u8; 2];
    a[t.f(1)]
}

fn main() with {} {
    let _ = g(&X {});
}
// error: `j` can't be `mut` in `f`
