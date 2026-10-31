#[derive(Copy, Discard)]
export struct X {}

impl q::Tr2 for X {
    fn two<N: q::Tr1, K: Discard>(self, n: N, k: K) { n.one(k) }
}
