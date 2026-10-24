mod m {
    #[derive(Discard)]
    pub struct P { pub n: i64 }

    impl P {
        fn secret(&self) -> i64 { self.n }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = m::P { n: 1 };
    out.line(&m::P::secret(&p))?;
    Result::Ok(())
}
// error: method `secret` of `P` is private to `m`
