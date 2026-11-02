// A panic while working out a const's value is an error while compiling,
// so that a use of it never panics.
const BIG: u8 = 200;
const SUM: u8 = BIG + 100;
// error: evaluating `SUM` panicked: integer overflow

fn main() {
    let _ = SUM;
}
