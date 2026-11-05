// `#[derive(Debug)]` is core's, whatever's in scope: here a trait of the
// program's own is called `Debug`, and the derive, and `{x:?}`, still mean
// core's. Core's `Debug` isn't in the prelude, and needn't be imported.
trait Debug {
    fn mine(&self) -> i32;
}

#[derive(Debug, Discard, Eq)]
struct P { n: i32 }

impl Debug for P {
    fn mine(&self) -> i32 { self.n }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { n: 4 };
    out.line(&f"{p:?} {p.n}")?; // expect: P { n: 4 } 4
    out.line(&p.mine())?; // expect: 4
    assert_eq(&p, &P { n: 4 });
    Result::Ok(())
}
