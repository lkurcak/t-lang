// `fold` drops the iterator when it's done, so it needs `Self: Discard`.
struct Lines { left: int }

impl Iterator for Lines {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn main() {
    print(Lines { left: 3 }.fold(0, |a, b| a + b));
}
// error: `fold` needs its `Self` to implement `Discard`, and `Lines` doesn't
