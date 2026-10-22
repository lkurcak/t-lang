// A linear iterator can be iterated through `&mut`, and then used up by
// its owner: here, closed.
struct Lines { left: i64 }

impl Iterator for Lines {
    type Item = i64;
    effect e = panic;

    fn next(&mut self) -> Option<i64> with panic {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn close(lines: Lines) -> i64 {
    let Lines { left } = lines;
    left
}

fn main() with panic {
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
