// Tuples are equal types only if they're equal field by field.
fn f(_p: (i64, bool)) {}

fn main() {
    f((true, 1));
}
// error: expected `i64`, found `bool`
