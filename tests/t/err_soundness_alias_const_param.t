// `Below<8>` is below 8: 8 isn't.
type Below<const N: usize> = { i: usize | i < N };

fn main() {
    let _i: Below<8> = 8;
}
// error: can't prove `i < 8`
