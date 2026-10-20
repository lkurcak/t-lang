// A `for` drops its iterator when it ends, so it must be `Discard`; to keep
// a linear one, iterate over `&mut it`.
struct Lines { left: int }

impl Iterator for Lines {
    type Item = int;
    effect e = {};

    fn next(&mut self) -> Option<int> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn main() {
    for n in (Lines { left: 2 }) {
        print(n);
    }
}
// error: a `for` throws its iterator away when it ends, and `Lines` isn't `Discard`; to keep it, iterate over a mutable reference to it: `for x in &mut it`
