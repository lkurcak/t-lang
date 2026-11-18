// Two closures give the adapter's `U`: `next` gives the second's.
#[derive(Discard)]
struct Map2<I, F, G> { iter: I, f: F, g: G }

impl<I: Iterator, U, F: FnMut(I::Item) -> U, G: FnMut() -> U> Iterator for Map2<I, F, G> where U: Discard {
    type Item = U;
    effect e = I::e;

    fn next(&mut self) -> Option<U> with I::e {
        match self.iter.next() {
            Some(x) => {
                let _u = (self.f)(x);
                Some((self.g)())
            }
            None => None,
        }
    }
}

fn map2<I: Iterator, U, F: FnMut(I::Item) -> U, G: FnMut() -> U>(iter: I, f: F, g: G) -> Map2<I, F, G> where U: Discard {
    Map2 { iter, f, g }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in map2(0..100, |x: usize| x % 8, || 100) {
        let _ = a[r];
    }
}
// error: can't prove `i < self.len()`
