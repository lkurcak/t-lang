// A `while` loop can end without its `break`, by its condition, so `x` may
// have no value after it.
fn main() {
    let x;
    let mut i = 0;
    while i < 3 {
        i += 1;
        x = i;
        break;
    }
    print(x);
}
// error: use of `x`, which has no value here
