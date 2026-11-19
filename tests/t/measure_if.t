// A measure's body may say `if c { a } else { b }`: a function of the
// three, as `Zip`'s `len` is the shorter one's (docs/88-iterator-laws.md).
#[derive(Discard)]
struct Two { a: usize, b: usize }

impl Two {
    #[measure]
    fn least(&self) -> usize {
        if self.a <= self.b { self.a } else { self.b }
    }

    #[measure]
    fn big(&self) -> bool {
        if self.a > 10 { true } else { self.b > 10 }
    }
}

fn pick(t: &Two) -> { n: usize | n <= t.a && n <= t.b } { t.least() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = Two { a: 4, b: 9 };
    out.line(&pick(&t))?; // expect: 4
    let big: { v: bool | v } = (Two { a: 1, b: 20 }).big();
    out.line(&big)?; // expect: true
    Ok(())
}
