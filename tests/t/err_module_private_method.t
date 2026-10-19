mod m {
    #[derive(Discard)]
    pub struct P { pub n: int }

    impl P {
        fn secret(&self) -> int { self.n }
    }
}

fn main() {
    let p = m::P { n: 1 };
    print(p.secret());
}
// error: method `secret` of `P` is private to `m`
