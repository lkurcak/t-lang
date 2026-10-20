// A linear iterator can be iterated through `&mut`, and then used up by
// its owner: here, closed.
struct Lines { left: int }

impl Iterator for Lines {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn close(lines: Lines) -> int {
    let Lines { left } = lines;
    left
}

fn main() {
    let mut lines = Lines { left: 5 };
    for n in &mut lines {
        if n == 2 {
            break;
        }
        print(n);
    }
    // expect: 4
    // expect: 3
    print(close(lines)); // expect: 2
}
