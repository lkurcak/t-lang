// Iterators set their effects; generic code passes them on.
#[derive(Discard)]
struct Ones { left: i64 }

impl Iterator for Ones {
    type Item = i64;
    effect e = panic;

    fn next(&mut self) -> Option<i64> with panic {
        if self.left == 0 {
            Option::None
        } else {
            self.left = self.left - 1;
            Option::Some(1)
        }
    }
}

fn first<I: Iterator<Item = i64> + Discard>(mut it: I) -> Option<i64> with I::e { it.next() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&first(Ones { left: 1 }).unwrap())?; // expect: 1
    // A range's `next` can't panic, so neither can this call.
    out.line(&first(0..3).is_some())?; // expect: true
    out.line(&(0..4).map(|x| x * 2i64).fold(0, |a, b| a + b))?; // expect: 12
    Result::Ok(())
}
