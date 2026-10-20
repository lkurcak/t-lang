// A `for` is `()`: it can end without a `break`, so a `break` can't give
// a value.
fn main() {
    let x = for i in 0..3 {
        break i;
    };
}
// error: a `break` with a value only works in a `loop`: a `for` loop can end without a `break`
