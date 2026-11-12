// A type parameter's bound can say what its associated effect may be at
// most: an upper bound, as every effect bound is.
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

fn first_maybe<I: Iterator<Item = u64, e = panic> + Discard>(mut it: I) -> Option<u64> with panic {
    it.next()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&first(0..3).unwrap())?;                // expect: 0
    out.line(&first_maybe(0..3).unwrap())?;          // expect: 0
    out.line(&first_maybe(Down { n: 3 }).unwrap())?; // expect: 2
    Result::Ok(())
}
