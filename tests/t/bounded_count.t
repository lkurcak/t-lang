// An iterator that says how many values it has yet to give at most, a
// `BoundedIterator`, can be counted without `panic`: `count` says `with
// panic unless Self: BoundedIterator`. A slice's is one, and so is what
// `filter`, `skip`, `map`, `rev`, `enumerate` and `zip` make of one
// (docs/92-where-a-bound-holds.md).
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

/// Counting needs no `panic` of any bounded iterator.
fn tally<I: BoundedIterator + Discard>(it: I) -> usize with I::e where I::Item: Discard {
    it.count()
}

/// But of one that may never end, it does.
#[allow(dead_code)]
fn loose<I: Iterator + Discard>(it: I) -> usize with {I::e, panic} where I::Item: Discard {
    it.count()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    let a = [1u8, 2, 3, 4, 5];
    out.line(&a.iter().count())?;                          // expect: 5
    out.line(&a.iter().filter(|x| **x > 2).count())?;      // expect: 3
    out.line(&a.iter().skip(2).count())?;                  // expect: 3
    out.line(&a.iter().skip(9).count())?;                  // expect: 0
    out.line(&a.iter().map(|x| *x + 1).rev().count())?;    // expect: 5
    out.line(&a.iter().zip(a.iter().skip(1)).count())?;    // expect: 4
    out.line(&tally(Down { n: 7 }))?;                      // expect: 7
    out.line(&tally((Down { n: 7 }).filter(|n| *n > 3)))?; // expect: 3
    Ok(())
}
