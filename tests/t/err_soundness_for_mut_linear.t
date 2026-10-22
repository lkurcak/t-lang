// `for x in &mut it` leaves `it` with its owner, who must still use it up.
struct Lines { left: i64 }

impl Iterator for Lines {
    type Item = i64;
    effect e = {};

    fn next(&mut self) -> Option<i64> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn main() {
    let mut lines = Lines { left: 2 };
    for n in &mut lines {
        print(n);
    }
}
// error: `lines` is never used up, and its type, `Lines`, isn't `Discard`
