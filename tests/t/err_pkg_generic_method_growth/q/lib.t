export trait Tr1 {
    fn one<K: Discard>(self, k: K);
}

export trait Tr2 {
    fn two<N: Tr1, K: Discard>(self, n: N, k: K);
}
