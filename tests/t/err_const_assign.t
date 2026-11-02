// A const is a value, not a place.
const N: u8 = 3;

fn main() {
    N = 4;
// error: can't assign to
}
