// An inherent method can have refinements too.
#[derive(Copy, Discard)]
struct Meter { n: i64 }

impl Meter {
    fn split(&self, parts: { p: i64 | p > 0 }) -> i64 { self.n / parts }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let m = Meter { n: 12 };
    out.line(&m.split(4))?; // expect: 3
    Result::Ok(())
}
