// `Down`'s `next` may panic, which `first`'s bound says it can't.
#[derive(Discard)]
struct Down { n: u64 }

impl Iterator for Down {
    type Item = u64;
    effect e = panic;

    fn next(&mut self) -> Option<u64> with panic {
        if self.n == 0 {
            return None;
        }
        self.n = self.n - 1;
        Some(self.n)
    }
}

fn first<I: Iterator<Item = u64, e = {}> + Discard>(mut it: I) -> Option<u64> { it.next() }

fn main() {
    let _ = first(Down { n: 3 });
}
// error: `<Down as Iterator>::e` may panic, but `first` needs `I::e = {}`
