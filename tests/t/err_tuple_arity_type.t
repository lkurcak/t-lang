// Tuples of different lengths are different types.
fn main() {
    let _x: (u8, u8) = (1u8, 2u8, 3u8);
}
// error: expected `(u8, u8)`, found `(u8, u8, u8)`
