// An iterator that overrides a default method can give anything through
// it: here `any` gives its closure `9`, which `next` never gives, and an
// adapter that gets its values through `any` gives that on.
#[derive(Discard)]
struct Low { n: usize }

impl Iterator for Low {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<{ v: usize | v < 8 }> {
        if self.n < 8 {
            let v = self.n;
            self.n = self.n + 1;
            Some(v)
        } else {
            None
        }
    }

    fn any<effect px, P: FnMut(usize) -> bool with px + Discard>(&mut self, mut p: P) -> bool
    with px
    {
        if self.n < 8 {
            self.n = self.n + 1;
            p(9)
        } else {
            false
        }
    }
}

/// Gets each value through `any`.
#[derive(Discard)]
struct Grab<I> { iter: I }

impl<I: Iterator> Iterator for Grab<I> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        let mut got = None;
        let _ = self.iter.any(|x| {
            got = Some(x);
            true
        });
        got
    }
}

fn grab<I: Iterator>(iter: I) -> Grab<I> where I::Item: Discard { Grab { iter } }

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in grab(Low { n: 0 }) {
        let _ = a[r];
    }
}
// error: can't prove
