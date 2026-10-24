// `main` can give back a `Result<(), E>`: an `Err` is written out where
// errors go, and the program fails.

#[derive(Discard)]
struct Oops { n: i64 }

impl core::fmt::Display for Oops {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.put(&"oops ")?;
        w.put(&self.n)
    }
}

fn main(os: std::Os) -> Result<(), Oops> {
    let mut out = os.stdout;
    let _ = out.line(&1i64); // expect: 1
    Result::Err(Oops { n: 2 })
}
// stderr: error: oops 2
