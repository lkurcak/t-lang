// `fold` drops the iterator when it's done, so it needs `Self: Discard`.
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

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(Lines { left: 3 }.fold(0, |a, b| a + b)))?;
    Result::Ok(())
}
// error: `fold` needs its `Self` to implement `Discard`, and `Lines` doesn't
