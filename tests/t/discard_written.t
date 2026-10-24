// A type can say how it's thrown away, with a `discard` of its own: it's
// called wherever a value of it is dropped, at the end of its scope, when
// it's overwritten, and on the way out of a `?`. Inner scopes end first.
struct Noisy<'a> { out: &'a mut std::io::Stdout, n: i64 }

impl<'a> Discard for Noisy<'a> {
    fn discard(self) {
        let _ = self.out.line(&self.n);
    }
}

fn fails() -> Result<(), i64> {
    Result::Err(0)
}

fn early(out: &mut std::io::Stdout, n: i64) -> Result<(), i64> {
    let _noisy = Noisy { out, n };
    fails()?;
    Result::Ok(())
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    {
        let a = Noisy { out: &mut out, n: 1 };
        {
            let _b = Noisy { out: &mut *a.out, n: 2 };
        } // expect: 2
    } // expect: 1
    let _ = early(&mut out, 3); // expect: 3
    let mut c = Noisy { out: &mut out, n: 4 };
    c.n = 5;
    let _d = c; // expect: 5
    Result::Ok(())
}
