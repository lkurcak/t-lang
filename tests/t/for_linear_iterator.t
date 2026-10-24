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

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut lines = Lines { left: 5 };
    let w = show(&mut out, &mut lines);
    let left = close(lines);
    w?;
    // expect: 4
    // expect: 3
    out.line(&left) // expect: 2
}

fn show(out: &mut std::io::Stdout, lines: &mut Lines) -> Result<(), std::io::Error> with panic {
    for n in lines {
        if n == 2 {
            break;
        }
        out.line(&n)?;
    }
    Result::Ok(())
}
