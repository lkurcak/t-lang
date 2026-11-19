// A trait's method's result's type argument naming a parameter is a
// promise about it too: an impl's can't make it `mut`.
trait Tr {
    fn f(&self, k: usize) -> Option<{ n: usize | n <= k }>;
}

#[derive(Discard)]
struct X {}

impl Tr for X {
    fn f(&self, mut k: usize) -> Option<usize> {
        k = 100;
        Some(k)
    }
}

fn g<T: Tr>(t: &T) -> u8 {
    let a = [7u8; 2];
    match t.f(1) {
        Some(n) => a[n],
        None => 0,
    }
}

fn main() with {} {
    let _ = g(&X {});
}
// error: `k` can't be `mut` in `f`
