// A tuple's values are values: none can be `str` or a slice.
fn f(_t: &(str, i64)) {}

fn main() {}
// error: `str` can only be behind a shared reference, as in `&str`
