fn dangle(n: i64) -> &'static i64 { &n }

fn main() {}
// error: `n` doesn't live long enough: it goes away when the function returns
