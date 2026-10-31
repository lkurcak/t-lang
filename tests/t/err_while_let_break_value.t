// A `while let` can end because its pattern doesn't match, with no value
// to give, so it's a `()`, as a `while` is.
fn main() {
    let mut it = 0..3i64;
    let _x = 'a: while let Option::Some(k) = it.next() {
        while let 0..=1 = k {
            break 'a 5;
        }
    };
}
// error: a `break` with a value only works in a `loop`: a `while let` loop can end without a `break`
