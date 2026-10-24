// An impl sets its associated effect, and its methods can't do more:
// generic code calling `next` knows only `Self::e`, here nothing.
#[derive(Discard)]
struct Down { n: i64 }

impl Iterator for Down {
    type Item = i64;
    effect e = {};

    fn next(&mut self) -> Option<i64> with Self::e {
        let n = self.n;
        self.n = n - 1;
        Option::Some(n)
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut d = Down { n: 3 };
    out.line(&d.next().is_some())?;
    Result::Ok(())
}
// error: `-` may panic, but `next` doesn't say `with panic`
