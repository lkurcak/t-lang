// A tuple's field is its index, written as plain digits.
fn main() {
    let p = (1i64, true);
    let _x = p.0u8;
}
// error: a tuple's field is written as plain digits, as in `.0`
