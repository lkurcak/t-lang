// Outside its module a measure is a symbol of the value, and a write to a
// field makes a new value: what was known of the old one isn't of it.
mod m {
    #[derive(Copy, Discard)]
    pub struct W { pub n: usize }

    impl W {
        #[measure]
        pub fn len(&self) -> usize { self.n }
    }

    pub fn at(w: &W, i: { i: usize | i < w.len() }) -> usize { i }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut w = m::W { n: 3 };
    if w.len() > 0 {
        w.n = 0;
        out.line(&m::at(&w, 0))?;
    }
    Result::Ok(())
}
// error: can't prove `i < w.len()` for argument `i` of `at`
