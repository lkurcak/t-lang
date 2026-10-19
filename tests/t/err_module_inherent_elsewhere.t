mod a {
    #[derive(Discard)]
    pub struct P { pub n: int }
}

impl a::P {
    fn n(&self) -> int { self.n }
}

fn main() {}
// error: an inherent impl of `P` must be in its module, `a`
