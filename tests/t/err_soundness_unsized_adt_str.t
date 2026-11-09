// Nor `str`.
struct Holder<'a, T> { r: &'a T }

fn f<'a>(h: Holder<'a, str>) {}

fn main() {}
// error: `str` can only be behind a shared reference, as in `&str`
