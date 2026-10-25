// `head < N` is a field's invariant: `N` itself breaks it.
struct Ring<const N: usize> { buf: [i64; N], head: { h: usize | h < N } }

fn full<const N: usize>() -> Ring<N> { Ring { buf: [0; N], head: N } }

fn main() {}
// error: can't prove `h < N` for field `head` of `Ring`
