// A field's refinement that ends `where I: Trait` is what a value of the
// struct has where the bound holds of its types: `Counted`'s index can't
// overflow where its iterator says how many values it has at most, so
// `step` can't panic there (docs/93-invariants-where.md).
use core::num::greatest;

/// Counts down: it has `n` values left.
#[derive(Discard)]
struct Down { n: usize }

impl Iterator for Down {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<usize> {
        if self.n == 0 {
            return None;
        }
        self.n = self.n - 1;
        Some(self.n)
    }
}

impl BoundedIterator for Down {
    #[measure]
    fn most(&self) -> usize { self.n }
}

/// Each value of `iter` with its index, which can't overflow where `iter`
/// says how many it has at most.
#[derive(Discard where I: Discard)]
struct Counted<I> {
    iter: I,
    next: { n: usize | n + iter.most() <= greatest::<usize>() where I: BoundedIterator },
}

/// One that never ends, and doesn't say how many it has.
#[derive(Discard)]
struct Ones {}

impl Iterator for Ones {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> { Some(1) }
}

impl<I: Iterator> Counted<I> {
    fn new(iter: I) -> Counted<I> { Counted { iter, next: 0 } }

    fn step(&mut self) -> Option<(usize, I::Item)> with {I::e, panic unless I: BoundedIterator} {
        match self.iter.next() {
            Some(x) => {
                let i = self.next;
                self.next = i + 1;
                Some((i, x))
            },
            None => None,
        }
    }
}

/// Of a type that's known not to meet the bound, a value has no such
/// refinement, whatever its index is.
fn far() -> Counted<Ones> { Counted { iter: Ones {}, next: greatest::<usize>() } }

fn bounded(n: usize) -> usize with {} {
    let mut c = Counted::new(Down { n });
    let mut last = 0;
    while let Some((i, _x)) = c.step() {
        last = i;
    }
    last
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&bounded(4))?; // expect: 3
    let mut ones = Counted::new(Ones {});
    let _a = ones.step();
    match ones.step() {
        Some((i, x)) => out.line(&f"{i} {x}")?, // expect: 1 1
        None => {},
    }
    let f = far();
    out.line(&(f.next == greatest::<usize>()))?; // expect: true
    Ok(())
}
