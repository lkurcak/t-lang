// A length is a `usize`, so a const written as one must be too.
const N: i64 = 2;

struct Pair { a: [u8; N] }
// error: `N` is a const of type `i64`, but a length is a `usize`

fn main() {}
