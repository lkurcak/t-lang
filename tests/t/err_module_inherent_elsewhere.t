mod a {
    #[derive(Discard)]
    pub struct P { pub n: i64 }
}

impl a::P {
    fn n(&self) -> i64 { self.n }
}

fn main() {}
// error: an inherent impl of `P` must be in its module, `a`
