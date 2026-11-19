// What a trait's method `ensures` names its parameters' values as passed:
// an impl's can't make one it names `mut`.
trait Tr {
    fn f(&self, k: usize, r: &mut usize) ensures *r <= k;
}

#[derive(Discard)]
struct X {}

impl Tr for X {
    fn f(&self, mut k: usize, r: &mut usize) {
        k = 100;
        *r = k;
    }
}

fn g<T: Tr>(t: &T) -> u8 {
    let a = [7u8; 2];
    let mut x: usize = 0;
    t.f(1, &mut x);
    a[x]
}

fn main() with {} {
    let _ = g(&X {});
}
// error: `k` can't be `mut` in `f`
