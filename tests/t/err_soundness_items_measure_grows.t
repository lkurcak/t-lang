// A bound that moves the wrong way bounds nothing: `hi` grows with each
// value, so `v < old(self.hi())` says nothing of the values after the
// first, past where `hi` started.
#[derive(Discard)]
struct Up { at: usize, hi: usize }

impl Up {
    fn new(at: usize, hi: usize) -> { u: Up | u.at() == at && u.hi() == hi } { Up { at, hi } }

    #[measure]
    fn at(&self) -> usize { self.at }

    #[measure]
    fn hi(&self) -> usize { self.hi }
}

impl Iterator for Up {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<{ v: usize | v < old(self.hi()) }>
    ensures
        self.hi() >= old(self.hi()),
    {
        if self.at < self.hi && self.hi < 100 {
            let v = self.at;
            self.at = self.at + 2;
            self.hi = self.hi + 1;
            Some(v)
        } else {
            None
        }
    }
}

fn main() {
    let a: [u8; 4] = [0; 4];
    for r in Up::new(0, 4) {
        let _ = a[r];
    }
}
// error: can't prove
