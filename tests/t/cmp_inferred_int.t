// What a comparison is waits for its operands' type: here they turn out to
// be integers, so it's the machine's `<`, not a call of `Ord::lt`, as
// `t mono` shows.
fn main() {
    let less = |a, b| a < b;
    print_bool(less(1i64, 2)); // expect: true
    print_bool(less(2i64, 1)); // expect: false
}
