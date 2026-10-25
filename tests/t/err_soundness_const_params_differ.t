// Two const parameters are two lengths the caller picks, maybe different.
fn f<const N: usize, const M: usize>(a: [i64; N]) -> [i64; M] { a }

fn main() {}
// error: expected `[i64; M]`, found `[i64; N]`
