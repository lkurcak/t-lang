// A `break` with a value in a `loop` inside a `while let` is that loop's;
// one for the `while let`, past an inner `while let`, isn't.
fn main() {
    let mut it = 0..3i64;
    while let Option::Some(_) = it.next() {
        let _ = loop {
            break 1i64;
        };
        while let Option::Some(_) = it.next() {
            break;
        }
        break 7;
    }
}
// error: a `break` with a value only works in a `loop`: a `while let` loop can end without a `break`
