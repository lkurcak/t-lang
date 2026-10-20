// An impl sets its associated effect, and its methods can't do more:
// generic code calling `next` knows only `Self::e`, here nothing.
#[derive(Discard)]
struct Down { n: int }

impl Iterator for Down {
    type Item = int;
    effect e = {};

    fn next(&mut self) -> Option<int> with Self::e {
        let n = self.n;
        self.n = n - 1;
        Option::Some(n)
    }
}

fn main() {
    let mut d = Down { n: 3 };
    print_bool(d.next().is_some());
}
// error: `-` may panic, but `next` doesn't say `with panic`
