// Nor after `&mut`: changing a string's bytes could make them not UTF-8,
// and nothing makes a `&mut str` anyway.
fn f(s: &mut str) {}

fn main() {}
// error: `str` can only be behind a shared reference, as in `&str`
