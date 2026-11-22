// A struct's own bounds hold in its fields' refinements, so one can call
// a measure of a trait a type parameter is bound by; and a field a
// refinement names can be lent to a trait's method, as to a function,
// when the reference can't outlive the call: the refinement is proved
// again after (docs/93-invariants-where.md).
use core::num::greatest;

#[derive(Discard where I: Discard)]
struct Numbered<I: BoundedIterator> {
    iter: I,
    next: { n: usize | n + iter.most() <= greatest::<usize>() },
}

impl<I: BoundedIterator> Numbered<I> {
    fn new(iter: I) -> Numbered<I> { Numbered { iter, next: 0 } }

    fn step(&mut self) -> Option<(usize, I::Item)> with I::e {
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

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    let a = [5u8, 6, 7];
    let mut e = Numbered::new(a.iter());
    let _a = e.step();
    match e.step() {
        Some((i, x)) => out.line(&f"{i} {x}")?, // expect: 1 6
        None => {},
    }
    Ok(())
}
