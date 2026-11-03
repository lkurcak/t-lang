// A path a package can't start, as it doesn't list the library, in a
// program with a const as a length: an error, not a crash (a found bug:
// resolving stopped before every function had a symbol).
const SIZE: usize = 2;

fn first(a: [u8; SIZE]) -> u8 { a[0] }

fn main(os: std::Os) {} // error: this package's `t.toml` doesn't list the `std` library
