// `str` is a type only right after `&`.
fn f(s: str) {}

fn main() {}
// error: `str` can only be behind a shared reference, as in `&str`
